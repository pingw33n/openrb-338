//! The standard mod (task-036): the embedded `assets/Standard.rbm` (task-067), and the mod
//! built from the CD's `ReBirth.dll` and `Rebirth.exe` (taken from the CD image, `RB_CD_ISO`, by
//! `rb-testkit`; nothing of them is written to the repository, and the built mod lives in
//! memory). The tests that need them skip without the image.

use rb_engine::samples::{self, SampleSet, pe};
use rbm::*;

/// `ReBirth.dll` or `Rebirth.exe` of the CD image (`RB_CD_ISO`, by `rb-testkit`), or `None` (a
/// skip with the message) when it is not set.
fn original(name: &str) -> Option<Vec<u8>> {
    let cd = rb_testkit::cd_files()?;
    let p = match name {
        "ReBirth.dll" => &cd.dll,
        "Rebirth.exe" => &cd.exe,
        _ => panic!("{name}: not a file of the CD helper"),
    };
    Some(std::fs::read(p).unwrap())
}

/// The layout every standard mod has (docs/rbm.md 13.2), whatever exe it was built from.
fn check_layout(bytes: &[u8]) -> (Rbm, KeyTable) {
    let r = Rbm::parse(bytes.to_vec()).unwrap();
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
    assert_eq!(info.name, "Standard ReBirth");
    assert_eq!(info.version, Some((1, 0, 0)));
    assert!(info.text.contains("rb mod pack-standard"), "{}", info.text);
    assert!(r.is_standard());

    // scrambled against the embedded window: loads without RB20FUL.DAT
    assert_eq!(r.key_pages(), [embedded::WINDOW; 2]);
    let t = KeyTable::embedded(&r).unwrap();
    assert!(r.all_covered(&t));
    for id in samples::FIRST_ID..=samples::LAST_ID {
        let name = slot_name(id).unwrap();
        assert_eq!(r.reader(r.find(name).unwrap()), Reader::Dll, "{name}");
    }
    for n in 0..58 {
        assert!(r.chunks()[2 + n].name.as_deref().unwrap().ends_with(".jpg"));
    }
    assert!(r.find("splash.jpg").is_some());
    assert!(r.find("Demo.rbs").is_none());
    // recrypting it again changes nothing
    assert_eq!(recrypt(&r, &t).unwrap(), bytes);
    (r, t)
}

#[test]
fn the_embedded_mod_is_a_standard_mod() {
    let (r, t) = check_layout(standard::EMBEDDED);
    assert_eq!(standard::EMBEDDED.len(), 2_572_792);
    // the built-in kit is the embedded mod's 46 samples
    assert_eq!(
        *standard::builtin_samples(),
        r.load_all_samples(&t).unwrap()
    );
    assert_eq!(
        standard::builtin_file("SPLASH.JPG").unwrap(),
        r.file_by_name("splash.jpg", &t).unwrap()
    );
}

#[test]
fn the_standard_mod_holds_the_builtin_kit_and_panel() {
    let (Some(dll), Some(exe)) = (original("ReBirth.dll"), original("Rebirth.exe")) else {
        return;
    };
    let std_mod = standard::build(&dll, &exe).unwrap();
    assert_eq!(std_mod.jpegs, 59);
    let (r, t) = check_layout(&std_mod.bytes);

    // really scrambled: a sample's stored bytes past 1024 are not the resource's
    let bd = r.find("TR808BD.aif").unwrap();
    let aiff = pe::named_type_resources(&dll, "AIFF").unwrap();
    let res = |id: u16| aiff.iter().find(|x| x.id == id).unwrap().data;
    assert_ne!(&r.raw_payload(bd)[bd.name_len..], res(1000));

    // the 46 samples, byte for byte the DLL's resources, under the lookup's names
    for id in samples::FIRST_ID..=samples::LAST_ID {
        let name = slot_name(id).unwrap();
        assert_eq!(r.file_by_name(name, &t).unwrap(), res(id), "{name}");
    }
    let set = r.load_all_samples(&t).unwrap();
    assert_eq!(set, SampleSet::from_dll(&dll).unwrap());
    // the embedded kit has the same samples
    assert_eq!(*standard::builtin_samples(), set);

    // the panel: the exe's JPEG resources 12015..12571 as <id>.jpg, 903 as splash.jpg
    let jpeg = pe::named_type_resources(&exe, "JPEG").unwrap();
    let mut n = 0;
    for j in jpeg.iter().filter(|j| j.id >= standard::FIRST_PANEL_ID) {
        let name = format!("{}.jpg", j.id);
        assert_eq!(r.file_by_name(&name, &t).unwrap(), j.data);
        // the CD's exe, of the embedded mod, has the same panel (docs/rbm.md 13.1)
        assert_eq!(standard::builtin_file(&name).unwrap(), j.data);
        n += 1;
    }
    assert_eq!(n, 58);
    let splash = jpeg.iter().find(|j| j.id == standard::SPLASH_ID).unwrap();
    assert_eq!(r.file_by_name("Splash.jpg", &t).unwrap(), splash.data);

    // the same files give the same mod
    assert_eq!(standard::build(&dll, &exe).unwrap().bytes, std_mod.bytes);
}

#[test]
fn a_mod_without_every_slot_is_no_standard_mod() {
    let (Some(dll), Some(exe)) = (original("ReBirth.dll"), original("Rebirth.exe")) else {
        return;
    };
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
