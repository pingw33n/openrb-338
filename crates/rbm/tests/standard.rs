//! The standard mod (task-036): built from `original/ReBirth.dll` and `original/Rebirth.exe`
//! (nothing of them is written to the repository; the mod lives in memory).

use std::path::PathBuf;

use rb_engine::samples::{self, SampleSet, pe};
use rbm::*;

fn original(name: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../original")
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn the_standard_mod_holds_the_builtin_kit_and_panel() {
    let (dll, exe) = (original("ReBirth.dll"), original("Rebirth.exe"));
    let std_mod = standard::build(&dll, &exe).unwrap();
    assert_eq!(std_mod.jpegs, 59);
    let r = Rbm::parse(std_mod.bytes.clone()).unwrap();
    assert!(r.warnings().is_empty());

    // HEAD, the recrypt pad, 59 JPEGs, 46 samples, INFO
    let tags: Vec<&[u8; 4]> = r.chunks().iter().map(|c| &c.tag).collect();
    assert_eq!(r.chunks().len(), 1 + 1 + 59 + 46 + 1);
    assert_eq!(tags[0], b"HEAD");
    assert_eq!(r.chunks()[0].size, 256);
    assert_eq!(r.chunks()[1].name.as_deref(), Some(PAD_NAME));
    assert_eq!(*tags.last().unwrap(), b"INFO");
    let info = r.info().unwrap();
    assert_eq!(info.name, standard::NAME);
    assert_eq!(info.version, Some((1, 0, 0)));
    assert!(r.is_standard());

    // scrambled against the embedded window: loads without RB20FUL.DAT
    assert_eq!(r.key_pages(), [embedded::WINDOW; 2]);
    let t = KeyTable::embedded(&r).unwrap();
    assert!(r.all_covered(&t));
    // and really scrambled: a sample's stored bytes past 1024 are not the resource's
    let bd = r.find("TR808BD.aif").unwrap();
    let aiff = pe::named_type_resources(&dll, "AIFF").unwrap();
    let res = |id: u16| aiff.iter().find(|x| x.id == id).unwrap().data;
    assert_ne!(&r.raw_payload(bd)[bd.name_len..], res(1000));

    // the 46 samples, byte for byte the DLL's resources, under the lookup's names
    for id in samples::FIRST_ID..=samples::LAST_ID {
        let name = slot_name(id).unwrap();
        assert_eq!(r.file_by_name(name, &t).unwrap(), res(id), "{name}");
        assert_eq!(r.reader(r.find(name).unwrap()), Reader::Dll);
    }
    let set = r.load_all_samples(&t).unwrap();
    assert_eq!(set, SampleSet::from_dll(&dll).unwrap());

    // the panel: the exe's JPEG resources 12015..12571 as <id>.jpg, 903 as splash.jpg
    let jpeg = pe::named_type_resources(&exe, "JPEG").unwrap();
    let mut n = 0;
    for j in jpeg.iter().filter(|j| j.id >= standard::FIRST_PANEL_ID) {
        assert_eq!(
            r.file_by_name(&format!("{}.jpg", j.id), &t).unwrap(),
            j.data
        );
        n += 1;
    }
    assert_eq!(n, 58);
    let splash = jpeg.iter().find(|j| j.id == standard::SPLASH_ID).unwrap();
    assert_eq!(r.file_by_name("Splash.jpg", &t).unwrap(), splash.data);
    assert!(r.find("Demo.rbs").is_none());

    // the same files give the same mod; recrypting it again changes nothing
    assert_eq!(standard::build(&dll, &exe).unwrap().bytes, std_mod.bytes);
    assert_eq!(recrypt(&r, &t).unwrap(), std_mod.bytes);
}

#[test]
fn a_mod_without_every_slot_is_no_standard_mod() {
    let (dll, exe) = (original("ReBirth.dll"), original("Rebirth.exe"));
    // the exe has no AIFF resources, the DLL no JPEGs
    assert!(matches!(
        standard::build(&exe, &exe),
        Err(RbmError::Standard(_))
    ));
    assert!(matches!(
        standard::build(&dll, &dll),
        Err(RbmError::Standard(_))
    ));
}
