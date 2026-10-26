/*
 * rbhost: an offline host for the original ReBirth.dll (ReBirth RB-338 2.01).
 *
 *   rbhost.exe [options] SONG.rbhm OUT.wav
 *
 * It loads ReBirth.dll into its own process, plays the panel's side of the shared-memory port that
 * Rebirth.exe would play, replays the song-load messages that tools/rbhost/rbsmsg.py derived from an
 * .rbs file (docs/song_load.md), renders idle blocks, runs the export sequence of
 * docs/export_state.md section 4 and writes a 16-bit stereo 44100 Hz WAV. No realtime thread, no
 * window, no Rebirth.exe. Everything runs on one thread; every message is dispatched by a 0-frame
 * SafeSoundCallback (the app's own pump without an audio channel, sub_408DF5) or by EngineIdle.
 *
 * Options:
 *   --dll PATH              ReBirth.dll to load (default C:\ReBirth RB-338\ReBirth.dll)
 *   --idle-blocks N         64-frame blocks rendered by the active, stopped song before the export
 *                           (as 1024-frame calls, like the app's MME stream; N a multiple of 16)
 *   --post-stop-blocks M    blocks rendered between the export's stop message and its locate
 *                           (the app renders a few realtime buffers there while it queries)
 *   --drum-counters A,B,C,D preset the process-global drum counters before anything runs:
 *                           808 snare word_1014CBD4, 909 snare word_1014E548,
 *                           808 bass-drum toggle word_1014CBD0, 909 bass-drum toggle word_1014E544
 *   --startup-sound         send top-level 58 {mod} after msg 52, as the app does when the
 *                           preference "Play Mod Startup Sound" is on
 *   --first-chunk-drain     leave locate and play in the port for the first export chunk to drain
 *                           (the app's way; default: dispatch them with 0-frame calls first)
 *   --no-untitled           do not create and activate an empty song before the song (the app
 *                           always has "Untitled song" active; docs/export_state.md section 12)
 *   --idle-fpu-cw HEX       experiment: the x87 control word during the idle blocks only
 *   --dump-tables DIR       write the run-time tables (investigation section 10 item 4) after the
 *                           song is created, with DIR/manifest.json
 *   --dump-state DIR        write the engine objects at export start, with DIR/manifest.json
 *   -v                      log every reply
 */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define DLL_BASE 0x10000000u
#define RING_SIZE 32000
#define REALTIME_FRAMES 1024
#define CHUNK_FRAMES 2048

typedef int(__cdecl *EngineOpenFn)(void);
typedef int(__cdecl *EngineIdleFn)(void);
typedef void(__cdecl *SafeSoundCallbackFn)(int16_t *buf, int frames, int x);

typedef struct {
    volatile LONG used;  /* bytes in the ring */
    volatile LONG free_; /* bytes free, 32000 at creation */
    volatile LONG rd;    /* read offset */
    volatile LONG wr;    /* write offset */
    unsigned char data[RING_SIZE];
} Ring; /* 0x7D10 bytes: sub_100179E0 */

typedef struct {
    Ring *ring;
    HANDLE sema;
    HANDLE event;
    const char *name;
} Port;

static HMODULE dll;
static EngineOpenFn engine_open;
static EngineIdleFn engine_idle;
static SafeSoundCallbackFn safe_cb;
static Port to_dll, from_dll;
static int verbose;
static int16_t pump_buf[CHUNK_FRAMES * 2];
static int16_t chunk_buf[CHUNK_FRAMES * 2];
static unsigned char reply[10000];
static int reply_len;

static void die(const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    fprintf(stderr, "rbhost: ");
    vfprintf(stderr, fmt, ap);
    fprintf(stderr, "\n");
    va_end(ap);
    exit(1);
}

static LONG WINAPI crash_filter(EXCEPTION_POINTERS *ep) {
    fprintf(stderr, "rbhost: exception 0x%08lx at %p\n", ep->ExceptionRecord->ExceptionCode,
            ep->ExceptionRecord->ExceptionAddress);
    fflush(stderr);
    ExitProcess(3);
    return EXCEPTION_EXECUTE_HANDLER;
}

/* An address of the DLL's image, from its IDA address (the image may be relocated). */
static void *va(uint32_t ida) { return (char *)dll + (ida - DLL_BASE); }

/* ------------------------------------------------------------------ x87 */

/* Rebirth.exe runs with 53-bit precision: _fpmath 0x4A7540 -> _setdefaultprecision 0x4AA860 ->
 * _controlfp(0x10000, 0x30000), control word 0x027F, which is also what new Win32 threads (the MME
 * worker) start with. The DLL never sets it. rbhost's own code uses SSE2 (-mfpmath=sse). */
static const uint16_t APP_FPU_CW = 0x027F;

static uint16_t get_cw(void) {
    uint16_t cw;
    __asm__ volatile("fnstcw %0" : "=m"(cw));
    return cw;
}

static void set_cw(uint16_t cw) { __asm__ volatile("fldcw %0" : : "m"(cw)); }

static void check_cw(const char *where) {
    uint16_t cw = get_cw();
    if (cw != APP_FPU_CW) {
        fprintf(stderr, "rbhost: x87 control word 0x%04x after %s, restored to 0x%04x\n", cw, where, APP_FPU_CW);
        set_cw(APP_FPU_CW);
    }
}

/* ------------------------------------------------------------------ the port */

static void port_open(Port *p, const char *name) {
    char buf[128];
    HANDLE map = OpenFileMappingA(FILE_MAP_ALL_ACCESS, FALSE, name);
    if (!map) die("OpenFileMapping(%s): %lu", name, GetLastError());
    p->ring = (Ring *)MapViewOfFile(map, FILE_MAP_ALL_ACCESS, 0, 0, 0);
    if (!p->ring) die("MapViewOfFile(%s): %lu", name, GetLastError());
    snprintf(buf, sizeof buf, "%s_SEMA", name);
    p->sema = OpenSemaphoreA(SEMAPHORE_ALL_ACCESS, FALSE, buf);
    if (!p->sema) die("OpenSemaphore(%s): %lu", buf, GetLastError());
    snprintf(buf, sizeof buf, "%s_EVENT", name);
    p->event = OpenEventA(EVENT_ALL_ACCESS, FALSE, buf);
    if (!p->event) die("OpenEvent(%s): %lu", buf, GetLastError());
    p->name = name;
}

static void port_lock(Port *p) {
    if (WaitForSingleObject(p->sema, 10000) != WAIT_OBJECT_0) die("lock %s", p->name);
}

static void port_unlock(Port *p) { ReleaseSemaphore(p->sema, 1, NULL); }

/* The writer of sub_10017F50. One thread only: a full ring is an error (it never fills, since every
 * message is pumped before the next one is written). */
static void port_write(Port *p, const unsigned char *msg, int len) {
    Ring *r = p->ring;
    int i;
    port_lock(p);
    if (len + 2 > r->free_) die("port %s full (%d bytes, %ld free)", p->name, len, (long)r->free_);
    for (i = 0; i < 2; i++) {
        r->data[r->wr] = (unsigned char)(len >> (8 * i));
        r->wr = (r->wr + 1) % RING_SIZE;
    }
    for (i = 0; i < len; i++) {
        r->data[r->wr] = msg[i];
        r->wr = (r->wr + 1) % RING_SIZE;
    }
    r->used += len + 2;
    r->free_ -= len + 2;
    port_unlock(p);
}

/* The reader of sub_10017D40: the next message's length, or -1 when the ring is empty. */
static int port_read(Port *p, unsigned char *buf, int max) {
    Ring *r = p->ring;
    int len = -1, i, pos;
    port_lock(p);
    if (r->used > 0) {
        pos = r->rd;
        len = r->data[pos];
        pos = (pos + 1) % RING_SIZE;
        len |= r->data[pos] << 8;
        pos = (pos + 1) % RING_SIZE;
        if (len > max) die("reply of %d bytes", len);
        for (i = 0; i < len; i++) {
            buf[i] = r->data[pos];
            pos = (pos + 1) % RING_SIZE;
        }
        r->rd = pos;
        r->used -= len + 2;
        r->free_ += len + 2;
        SetEvent(p->event);
    }
    port_unlock(p);
    return len;
}

/* ------------------------------------------------------------------ driving the engine */

static void call_safe(int16_t *buf, int frames) {
    safe_cb(buf, frames, 0);
    check_cw("SafeSoundCallback");
}

/* --idle-fpu-cw: an experiment, the x87 control word for the idle blocks only. */
static int idle_cw = -1;

/* Reads every reply waiting in "ReBirth DLL->Panel"; keeps the last one in reply[]. */
static int collect_replies(void) {
    int n = 0, len;
    unsigned char buf[10000];
    while ((len = port_read(&from_dll, buf, sizeof buf)) >= 0) {
        if (verbose) {
            int i;
            fprintf(stderr, "  reply type %d len %d:", len ? buf[0] : -1, len);
            for (i = 0; i < len && i < 16; i++) fprintf(stderr, " %02x", buf[i]);
            fprintf(stderr, "\n");
        }
        if (len > 0 && buf[0] == 75) die("engine replied 'K' (out of memory), code %d", len > 1 ? buf[1] : -1);
        memcpy(reply, buf, len);
        reply_len = len;
        n++;
    }
    return n;
}

/* Dispatches everything written to the engine: 0-frame SafeSoundCallback calls drain the port
 * (sub_1001CC40, at least one message per call), EngineIdle runs the queued types (52, 54..58, 63:
 * one per call), and one more 0-frame call forwards the queued replies (engine +0x41C). */
static int pump(void) {
    int guard = 0, i;
    while (to_dll.ring->used > 0) {
        call_safe(pump_buf, 0);
        if (++guard > 100000) die("the engine does not drain the port");
    }
    for (i = 0; i < 4; i++) {
        engine_idle();
        check_cw("EngineIdle");
    }
    call_safe(pump_buf, 0);
    return collect_replies();
}

static int send_msg(const unsigned char *msg, int len) {
    reply_len = 0;
    port_write(&to_dll, msg, len);
    return pump();
}

static void expect(int type, const char *what) {
    if (reply_len < 1 || reply[0] != type)
        die("%s: expected reply %d, got %d", what, type, reply_len ? reply[0] : -1);
}

static int slot;

static void song_msg(const unsigned char *payload, int len) {
    unsigned char m[16];
    m[0] = 78;
    m[1] = (unsigned char)slot;
    memcpy(m + 2, payload, len);
    send_msg(m, len + 2);
}

static void render_idle(long blocks) {
    if (idle_cw >= 0) {
        set_cw((uint16_t)idle_cw);
        while (blocks >= 16) {
            safe_cb(chunk_buf, REALTIME_FRAMES, 0);
            blocks -= 16;
        }
        if (blocks > 0) safe_cb(chunk_buf, (int)blocks * 64, 0);
        set_cw(APP_FPU_CW);
        return;
    }
    while (blocks >= 16) {
        call_safe(chunk_buf, REALTIME_FRAMES);
        blocks -= 16;
    }
    if (blocks > 0) call_safe(chunk_buf, (int)blocks * 64);
}

/* ------------------------------------------------------------------ files */

static unsigned char *read_file(const char *path, long *size) {
    FILE *f = fopen(path, "rb");
    unsigned char *data;
    if (!f) die("cannot open %s", path);
    fseek(f, 0, SEEK_END);
    *size = ftell(f);
    fseek(f, 0, SEEK_SET);
    data = malloc(*size ? *size : 1);
    if (fread(data, 1, *size, f) != (size_t)*size) die("cannot read %s", path);
    fclose(f);
    return data;
}

static uint32_t le32(const unsigned char *p) { return p[0] | p[1] << 8 | p[2] << 16 | (uint32_t)p[3] << 24; }

static void put_le32(unsigned char *p, uint32_t v) {
    p[0] = v;
    p[1] = v >> 8;
    p[2] = v >> 16;
    p[3] = v >> 24;
}

static void write_blob(const char *dir, const char *name, const void *data, uint32_t size) {
    char path[MAX_PATH];
    FILE *f;
    snprintf(path, sizeof path, "%s\\%s", dir, name);
    f = fopen(path, "wb");
    if (!f) die("cannot write %s", path);
    fwrite(data, 1, size, f);
    fclose(f);
}

typedef struct {
    FILE *f;
    int n;
} Manifest;

static void manifest_open(Manifest *m, const char *dir, const char *what) {
    char path[MAX_PATH];
    CreateDirectoryA(dir, NULL);
    snprintf(path, sizeof path, "%s\\manifest.json", dir);
    m->f = fopen(path, "w");
    if (!m->f) die("cannot write %s", path);
    m->n = 0;
    fprintf(m->f, "{\n  \"what\": \"%s\",\n  \"dll_base\": \"0x%08lx\",\n  \"items\": [\n", what,
            (unsigned long)(uintptr_t)dll);
}

/* address: the IDA address for DLL data, or the run-time heap address for objects */
static void manifest_item(Manifest *m, const char *dir, const char *name, uint32_t address, const void *data,
                          uint32_t size, const char *type, const char *shape, const char *note) {
    char file[MAX_PATH];
    snprintf(file, sizeof file, "%s.bin", name);
    write_blob(dir, file, data, size);
    fprintf(m->f, "%s    {\"name\": \"%s\", \"file\": \"%s\", \"address\": \"0x%08lx\", \"size\": %lu, "
                  "\"type\": \"%s\", \"shape\": %s, \"note\": \"%s\"}",
            m->n ? ",\n" : "", name, file, (unsigned long)address, (unsigned long)size, type, shape, note);
    m->n++;
}

static void manifest_close(Manifest *m) {
    fprintf(m->f, "\n  ]\n}\n");
    fclose(m->f);
}

/* ------------------------------------------------------------------ dumps */

typedef struct {
    const char *name;
    uint32_t addr, size;
    const char *type, *shape, *note;
} TableDef;

/* docs/initial_investigation.md section 10 item 4; builders in the notes of task-011. */
static const TableDef TABLES[] = {
    {"t", 0x10054550, 128 * 4, "f32", "[128]", "level curve T, sub_100046B0 (song ctor, once)"},
    {"pan", 0x10034550, 128 * 128 * 2 * 4, "f32", "[128,128,2]", "PAN[a][b] = {L,R}, sub_100046B0"},
    {"pitch303", 0x10125364, 6144 * 4, "f32", "[6144]", "303 pitch table, 303 ctor sub_10023920 (once)"},
    {"filt303", 0x1012B364, 8 * 1024 * 4 * 4, "f32", "[8,1024,4]", "303 FILT {c0,c1,c2,c3}, sub_100234A0"},
    {"pcf_lp", 0x1010C270, 8 * 512 * 3 * 4, "f32", "[8,512,3]", "PCF low-pass {c0,c1,c2}, sub_100172C4"},
    {"pcf_bp", 0x10100270, 8 * 512 * 3 * 4, "f32", "[8,512,3]", "PCF band-pass {c0,c1,c2}, sub_10017574"},
    {"dist_w", 0x100D47A4, 32 * 256 * 4, "f32", "[32,256]", "distortion waveshaper W, sub_10011210 (once)"},
    {"t808", 0x1014CBD8, 128 * 2, "u16", "[128]", "808 tom step T808, 808 ctor sub_10026F70 (first)"},
    {"t909", 0x1014E54C, 128 * 2, "u16", "[128]", "909 tom step T909, 909 ctor sub_1002A380 (first)"},
    {"dt909", 0x1014E64C, 16 * 4, "f32", "[16]", "909 decay table DT, 909 ctor sub_1002A380 (first)"},
    {"smooth303", 0x1012470C, 3 * 4, "f32", "[3]", "303 cutoff smoother {b2, A, g}, static init sub_10026BB0"},
    {"smooth_pcf", 0x10100260, 3 * 4, "f32", "[3]", "PCF cutoff smoother {b2, A, g}, static init sub_10017840"},
};

static void dump_tables(const char *dir) {
    Manifest m;
    size_t i;
    manifest_open(&m, dir, "tables");
    for (i = 0; i < sizeof TABLES / sizeof TABLES[0]; i++) {
        const TableDef *t = &TABLES[i];
        manifest_item(&m, dir, t->name, t->addr, va(t->addr), t->size, t->type, t->shape, t->note);
    }
    manifest_close(&m);
    fprintf(stderr, "rbhost: tables written to %s\n", dir);
}

typedef struct {
    uint32_t vtable;
    const char *name;
    uint32_t size;
} DevClass;

static const DevClass DEVICES[] = {
    {0x10033FD4, "tb303", 0xD04},  {0x10034154, "tr808", 0x1910}, {0x10034294, "tr909", 0x1920},
    {0x10032CB4, "delay", 0x74},   {0x10033144, "dist", 0x60},    {0x10033AD4, "pcf", 0x8C},
    {0x10032B24, "comp", 0x6C},
};

/* Drum voices: dword index in the device, count, object size (ctors sub_10026F70, sub_1002A380). */
typedef struct {
    int index, count;
    uint32_t size;
} VoiceGroup;

static const VoiceGroup VOICES808[] = {{1591, 5, 0x20}, {1596, 3, 0x30}, {1599, 3, 0x28}, {1602, 1, 0x2C}};
static const VoiceGroup VOICES909[] = {{1592, 4, 0x20}, {1596, 5, 0x30}, {1601, 2, 0x28}, {1603, 1, 0x2C}};

static void dump_voices(Manifest *m, const char *dir, const char *dev_name, const uint32_t *dev,
                        const VoiceGroup *g, int ng) {
    int i, j, k = 0;
    char name[64];
    for (i = 0; i < ng; i++)
        for (j = 0; j < g[i].count; j++, k++) {
            void *v = (void *)(uintptr_t)dev[g[i].index + j];
            snprintf(name, sizeof name, "%s_voice%02d", dev_name, k);
            if (v) manifest_item(m, dir, name, (uint32_t)(uintptr_t)v, v, g[i].size, "bytes", "null", "drum voice");
        }
}

static void dump_state(const char *dir) {
    Manifest m;
    char *global = *(char **)va(0x1011D0B0);
    char *engine = *(char **)(global + 0x370);
    char *transport = *(char **)va(0x1011D0B4);
    char *song = *(char **)(engine + 0x14 + 4 * slot);
    char *mixer = *(char **)(song + 0x170);
    int count = *(unsigned char *)(song + 0x1C4), i;
    char name[64];
    manifest_open(&m, dir, "state at export start");
    manifest_item(&m, dir, "engine", (uint32_t)(uintptr_t)engine, engine, 0x430, "bytes", "null",
                  "[dword_1011D0B0+0x370], EngineOpen");
    manifest_item(&m, dir, "transport", (uint32_t)(uintptr_t)transport, transport, 0x174, "bytes", "null",
                  "dword_1011D0B4");
    manifest_item(&m, dir, "song", (uint32_t)(uintptr_t)song, song, 0x1910, "bytes", "null", "song ctor sub_10011350");
    manifest_item(&m, dir, "mixer", (uint32_t)(uintptr_t)mixer, mixer, 0xA0, "bytes", "null", "song+0x170");
    for (i = 0; i < count; i++) {
        uint32_t *dev = *(uint32_t **)(song + 0x18C + 4 * i);
        uint32_t vt = dev ? dev[0] : 0;
        size_t c;
        const DevClass *dc = NULL;
        if (!dev || (char *)dev == mixer) continue;
        for (c = 0; c < sizeof DEVICES / sizeof DEVICES[0]; c++)
            if (vt == (uint32_t)(uintptr_t)va(DEVICES[c].vtable)) dc = &DEVICES[c];
        snprintf(name, sizeof name, "dev%d_%s", i, dc ? dc->name : "unknown");
        manifest_item(&m, dir, name, (uint32_t)(uintptr_t)dev, dev, dc ? dc->size : 0x100, "bytes", "null",
                      "song+0x18C list");
        if (dc && !strcmp(dc->name, "tr808")) dump_voices(&m, dir, name, dev, VOICES808, 4);
        if (dc && !strcmp(dc->name, "tr909")) dump_voices(&m, dir, name, dev, VOICES909, 4);
    }
    manifest_item(&m, dir, "drum_counters", 0x1014CBD0, va(0x1014CBD0), 8, "u16", "[4]",
                  "word_1014CBD0 808 BD toggle, word_1014CBD2, word_1014CBD4 808 snare, word_1014CBD6");
    manifest_item(&m, dir, "drum_counters909", 0x1014E544, va(0x1014E544), 8, "u16", "[4]",
                  "word_1014E544 909 BD toggle, word_1014E546, word_1014E548 909 snare, word_1014E54A");
    manifest_item(&m, dir, "delay_gate", 0x10054790, va(0x10054790), 1, "u8", "[1]", "byte_10054790");
    manifest_item(&m, dir, "delay_buffer", 0x10054794, va(0x10054794), 131072 * 4, "f32", "[131072]",
                  "flt_10054794, global delay line");
    manifest_close(&m);
    fprintf(stderr, "rbhost: state written to %s\n", dir);
}

/* ------------------------------------------------------------------ main */

static void usage(void) {
    fprintf(stderr,
            "usage: rbhost [--dll PATH] [--idle-blocks N] [--post-stop-blocks M] [--drum-counters A,B,C,D]\n"
            "              [--startup-sound] [--first-chunk-drain] [--no-untitled] [--idle-fpu-cw HEX]\n"
            "              [--dump-tables DIR] [--dump-state DIR] [-v]\n"
            "              SONG.rbhm OUT.wav\n");
    exit(2);
}

int main(int argc, char **argv) {
    const char *dll_path = "C:\\ReBirth RB-338\\ReBirth.dll";
    const char *msg_path = NULL, *wav_path = NULL, *tables_dir = NULL, *state_dir = NULL;
    long idle_blocks = 0, post_stop_blocks = 0;
    int drum[4], drum_set = 0, startup_sound = 0, first_chunk_drain = 0, untitled = 1, i;
    long size;
    unsigned char *data, m[16];
    uint32_t version, tempo, loop_start, loop_end, count, p;
    unsigned mod;
    FILE *wav;
    unsigned char header[44];
    uint32_t frames = 0;
    uint16_t cw0;
    DWORD t0 = GetTickCount();

    SetUnhandledExceptionFilter(crash_filter);
    for (i = 1; i < argc; i++) {
        const char *a = argv[i];
        if (!strcmp(a, "--dll") && i + 1 < argc) dll_path = argv[++i];
        else if (!strcmp(a, "--idle-blocks") && i + 1 < argc) idle_blocks = strtol(argv[++i], NULL, 10);
        else if (!strcmp(a, "--post-stop-blocks") && i + 1 < argc) post_stop_blocks = strtol(argv[++i], NULL, 10);
        else if (!strcmp(a, "--drum-counters") && i + 1 < argc) {
            if (sscanf(argv[++i], "%d,%d,%d,%d", &drum[0], &drum[1], &drum[2], &drum[3]) != 4) usage();
            drum_set = 1;
        } else if (!strcmp(a, "--startup-sound")) startup_sound = 1;
        else if (!strcmp(a, "--first-chunk-drain")) first_chunk_drain = 1;
        else if (!strcmp(a, "--no-untitled")) untitled = 0;
        else if (!strcmp(a, "--idle-fpu-cw") && i + 1 < argc) idle_cw = (int)strtol(argv[++i], NULL, 0);
        else if (!strcmp(a, "--dump-tables") && i + 1 < argc) tables_dir = argv[++i];
        else if (!strcmp(a, "--dump-state") && i + 1 < argc) state_dir = argv[++i];
        else if (!strcmp(a, "-v")) verbose = 1;
        else if (a[0] == '-') usage();
        else if (!msg_path) msg_path = a;
        else if (!wav_path) wav_path = a;
        else usage();
    }
    if (!msg_path || !wav_path || idle_blocks < 0 || post_stop_blocks < 0) usage();
    if (idle_blocks % 16 || post_stop_blocks % 16)
        fprintf(stderr, "rbhost: warning: block counts not a multiple of 16 (the app's MME calls are 16 blocks)\n");

    data = read_file(msg_path, &size);
    if (size < 26 || memcmp(data, "RBHM", 4)) die("%s is not a message file", msg_path);
    version = le32(data + 4);
    mod = data[8] | data[9] << 8;
    tempo = le32(data + 10);
    loop_start = le32(data + 14);
    loop_end = le32(data + 18);
    count = le32(data + 22);
    if (version != 1) die("message file version %u", version);

    cw0 = get_cw();
    set_cw(APP_FPU_CW);
    dll = LoadLibraryA(dll_path);
    if (!dll) die("LoadLibrary(%s): %lu", dll_path, GetLastError());
    check_cw("LoadLibrary");
    engine_open = (EngineOpenFn)GetProcAddress(dll, "EngineOpen");
    engine_idle = (EngineIdleFn)GetProcAddress(dll, "EngineIdle");
    safe_cb = (SafeSoundCallbackFn)GetProcAddress(dll, "SafeSoundCallback");
    if (!engine_open || !engine_idle || !safe_cb) die("missing exports");
    fprintf(stderr, "rbhost: %s at %p, x87 control word 0x%04x -> 0x%04x\n", dll_path, (void *)dll, cw0, APP_FPU_CW);
    if (drum_set) {
        *(uint16_t *)va(0x1014CBD4) = (uint16_t)drum[0];
        *(uint16_t *)va(0x1014E548) = (uint16_t)drum[1];
        *(uint16_t *)va(0x1014CBD0) = (uint16_t)drum[2];
        *(uint16_t *)va(0x1014E544) = (uint16_t)drum[3];
    }

    if (!engine_open()) die("EngineOpen failed");
    check_cw("EngineOpen");
    port_open(&to_dll, "ReBirth Panel->DLL");
    port_open(&from_dll, "ReBirth DLL->Panel");

    /* Start-up handshake (exe sub_40FAA9): top-level 50, reply '@' with 6 bytes. */
    m[0] = 50;
    send_msg(m, 1);
    expect(64, "msg 50");
    /* The built-in mod (exe sub_41693C): top-level 56 {u16 id}, reply 'C'. */
    m[0] = 56;
    m[1] = mod >> 8;
    m[2] = mod & 0xFF;
    send_msg(m, 3);
    expect(67, "msg 56");
    /* The app's "Untitled song", active before the song is opened. Activating the opened song then
     * calls slot 6 (note-off, silence, delay clear) on the untitled song's devices; with no song
     * active, sub_1001BDA4 would call it on the new song's own devices instead, which changes the
     * 303 release constants (+0xCC8, +0xCCC) its idle blocks run with. */
    if (untitled) {
        m[0] = 52;
        send_msg(m, 3);
        expect(65, "msg 52 (untitled)");
        m[3] = 53;
        m[4] = reply[1];
        send_msg(m + 3, 2);
    }
    /* New song (exe sub_425A5D): top-level 52 {u16 mod}, reply 'A' {slot}. */
    m[0] = 52;
    send_msg(m, 3);
    expect(65, "msg 52");
    slot = reply[1];
    if (startup_sound) {
        m[0] = 58;
        send_msg(m, 3);
    }
    /* The song's load messages (docs/song_load.md section 2, step 5 and 6). */
    for (p = 26, i = 0; (uint32_t)i < count; i++) {
        uint32_t len = le32(data + p);
        unsigned char *msg = data + p + 4;
        if (p + 4 + len > (uint32_t)size || len < 3 || msg[0] != 78) die("bad message %d", i);
        msg[1] = (unsigned char)slot;
        send_msg(msg, (int)len);
        p += 4 + len;
    }
    if (tables_dir) dump_tables(tables_dir);
    /* Activate (exe sub_40D163 from the song window ctor): top-level 53 {slot}. */
    m[0] = 53;
    m[1] = (unsigned char)slot;
    send_msg(m, 2);

    render_idle(idle_blocks);

    /* The export, docs/export_state.md section 4 (sub_427988). */
    m[0] = 35, m[1] = 1, m[2] = 0;
    song_msg(m, 3); /* stop */
    render_idle(post_stop_blocks);
    {
        unsigned char locate[] = {78, (unsigned char)slot, 35, 8, loop_start >> 24, loop_start >> 16, loop_start >> 8,
                                  loop_start};
        unsigned char play[] = {78, (unsigned char)slot, 35, 1, 1};
        port_write(&to_dll, locate, sizeof locate);
        port_write(&to_dll, play, sizeof play);
        if (!first_chunk_drain) pump();
    }
    if (state_dir) dump_state(state_dir);

    wav = fopen(wav_path, "wb");
    if (!wav) die("cannot write %s", wav_path);
    memset(header, 0, sizeof header);
    fwrite(header, 1, sizeof header, wav);
    {
        /* sub_4280D5: pos in double, n = ftol((end - pos) / (tempo * (192.0 / 2646000000.0))), the last
         * chunk when n < 2048, pos += (tempo * n) * (192.0 / 2646000000.0). */
        double pos = (double)(int32_t)loop_start;
        int last = 0;
        while (!last) {
            int32_t n = (int32_t)(int64_t)(((double)(int32_t)loop_end - pos) /
                                           ((double)(int32_t)tempo * (192.0 / 2646000000.0)));
            double step;
            if (n >= CHUNK_FRAMES) n = CHUNK_FRAMES;
            else last = 1;
            step = ((double)(int32_t)tempo * (double)n) * (192.0 / 2646000000.0);
            call_safe(chunk_buf, n);
            if (n > 0) {
                fwrite(chunk_buf, 4, n, wav);
                frames += n;
            }
            pos = pos + step;
        }
    }
    /* After the loop: stop, msg 33, SafeSoundCallback(NULL, 0, 0) (0x427DA6..0x427DD6). */
    {
        unsigned char stop[] = {78, (unsigned char)slot, 35, 1, 0};
        unsigned char reset[] = {78, (unsigned char)slot, 33};
        port_write(&to_dll, stop, sizeof stop);
        port_write(&to_dll, reset, sizeof reset);
        pump();
    }
    memcpy(header, "RIFF", 4);
    put_le32(header + 4, 36 + frames * 4);
    memcpy(header + 8, "WAVEfmt ", 8);
    put_le32(header + 16, 16);
    header[20] = 1;  /* PCM */
    header[22] = 2;  /* channels */
    put_le32(header + 24, 44100);
    put_le32(header + 28, 44100 * 4);
    header[32] = 4;
    header[34] = 16;
    memcpy(header + 36, "data", 4);
    put_le32(header + 40, frames * 4);
    fseek(wav, 0, SEEK_SET);
    fwrite(header, 1, sizeof header, wav);
    fclose(wav);
    fprintf(stderr, "rbhost: slot %d, %u messages, idle %ld + %ld blocks, %u frames written to %s (%lu ms)\n",
            slot, count, idle_blocks, post_stop_blocks, frames, wav_path, (unsigned long)(GetTickCount() - t0));
    fflush(stderr);
    ExitProcess(0);
    return 0;
}
