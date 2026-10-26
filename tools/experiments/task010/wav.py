"""task-010 helpers: read the left channel of a 16-bit stereo WAV export."""
import struct


def left(path):
    """Left-channel int16 samples of a PCM WAV (the app's export format)."""
    with open(path, "rb") as f:
        d = f.read()
    pos = 12
    while pos + 8 <= len(d):
        tag, size = d[pos:pos + 4], struct.unpack("<I", d[pos + 4:pos + 8])[0]
        if tag == b"data":
            pcm = d[pos + 8:pos + 8 + size]
            v = struct.unpack("<%dh" % (len(pcm) // 2), pcm)
            return v[0::2]
        pos += 8 + size + (size & 1)
    raise ValueError("%s: no data chunk" % path)
