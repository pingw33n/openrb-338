//! A minimal PE (Portable Executable) resource reader: finds the resources of one named type in
//! a DLL image, with no Windows API. It reads the file as it is on disk (no loading, no
//! relocations): a resource's data is found through the section table.

use super::SampleError;

fn u16_at(d: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        d.get(o..o.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn u32_at(d: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        d.get(o..o.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn bad(msg: impl Into<String>) -> SampleError {
    SampleError::Pe(msg.into())
}

struct Section {
    rva: u32,
    /// The bytes the section occupies in memory that the file holds.
    size: u32,
    file_off: u32,
}

/// One resource: its numeric ID and its data, a slice of the file.
pub struct Resource<'a> {
    pub id: u16,
    pub data: &'a [u8],
}

struct Pe<'a> {
    d: &'a [u8],
    sections: Vec<Section>,
    /// File offset of the resource directory root.
    rsrc: usize,
    /// Size of the resource data directory, to bound the walk.
    rsrc_size: usize,
}

impl Pe<'_> {
    fn rva_to_off(&self, rva: u32) -> Option<usize> {
        let s = self
            .sections
            .iter()
            .find(|s| rva >= s.rva && rva - s.rva < s.size)?;
        Some((rva - s.rva) as usize + s.file_off as usize)
    }

    /// The entries of the directory at `rsrc + off`: (name-or-id field, offset field).
    fn entries(&self, off: u32) -> Result<Vec<(u32, u32)>, SampleError> {
        let base = self
            .rsrc
            .checked_add(off as usize)
            .ok_or_else(|| bad("resource directory offset overflows"))?;
        if off as usize >= self.rsrc_size {
            return Err(bad(
                "resource directory offset is outside the resource section",
            ));
        }
        let named = u16_at(self.d, base + 12).ok_or_else(|| bad("truncated resource directory"))?;
        let ids = u16_at(self.d, base + 14).ok_or_else(|| bad("truncated resource directory"))?;
        let n = named as usize + ids as usize;
        (0..n)
            .map(|i| {
                let o = base + 16 + 8 * i;
                Some((u32_at(self.d, o)?, u32_at(self.d, o + 4)?))
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| bad("truncated resource directory"))
    }

    /// A resource directory string (a u16 length, then UTF-16LE).
    fn name(&self, off: u32) -> Option<String> {
        let p = self.rsrc.checked_add(off as usize)?;
        let len = u16_at(self.d, p)? as usize;
        let raw = self.d.get(p + 2..p + 2 + 2 * len)?;
        let units: Vec<u16> = raw
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| u16::from_le_bytes(c))
            .collect();
        String::from_utf16(&units).ok()
    }
}

/// The resources of the named type `type_name` ("AIFF"), by ID, in directory order. For an ID
/// with several languages the first is taken (`FindResourceA` with a type and an ID only, as the
/// DLL calls it at 0x1000B620, does not choose a language either).
pub fn named_type_resources<'a>(
    d: &'a [u8],
    type_name: &str,
) -> Result<Vec<Resource<'a>>, SampleError> {
    if d.get(..2) != Some(b"MZ") {
        return Err(bad("not a PE file (no MZ header)"));
    }
    let pe = u32_at(d, 0x3C).ok_or_else(|| bad("truncated DOS header"))? as usize;
    if d.get(pe..pe.saturating_add(4)) != Some(b"PE\0\0") {
        return Err(bad("not a PE file (no PE signature)"));
    }
    let nsec = u16_at(d, pe + 6).ok_or_else(|| bad("truncated COFF header"))? as usize;
    let opt_size = u16_at(d, pe + 20).ok_or_else(|| bad("truncated COFF header"))? as usize;
    let opt = pe + 24;
    // The resource entry is data directory 2; the directories start after 96 (PE32) or 112
    // (PE32+) bytes of the optional header.
    let dirs = match u16_at(d, opt) {
        Some(0x10B) => opt + 96,
        Some(0x20B) => opt + 112,
        _ => return Err(bad("unknown optional header magic")),
    };
    let rsrc_rva = u32_at(d, dirs + 16).ok_or_else(|| bad("truncated optional header"))?;
    let rsrc_size = u32_at(d, dirs + 20).ok_or_else(|| bad("truncated optional header"))?;
    if rsrc_rva == 0 {
        return Err(bad("the file has no resource section"));
    }
    let mut sections = Vec::new();
    for i in 0..nsec {
        let o = opt + opt_size + 40 * i;
        let (vsize, rva, raw_size, raw_ptr) = (|| {
            Some((
                u32_at(d, o + 8)?,
                u32_at(d, o + 12)?,
                u32_at(d, o + 16)?,
                u32_at(d, o + 20)?,
            ))
        })()
        .ok_or_else(|| bad("truncated section table"))?;
        sections.push(Section {
            rva,
            size: vsize.max(raw_size),
            file_off: raw_ptr,
        });
    }
    let mut p = Pe {
        d,
        sections,
        rsrc: 0,
        rsrc_size: rsrc_size as usize,
    };
    p.rsrc = p
        .rva_to_off(rsrc_rva)
        .ok_or_else(|| bad("the resource directory is outside every section"))?;

    let mut out = Vec::new();
    for (name, off) in p.entries(0)? {
        if name & 0x8000_0000 == 0 || p.name(name & 0x7FFF_FFFF).as_deref() != Some(type_name) {
            continue;
        }
        if off & 0x8000_0000 == 0 {
            return Err(bad("a resource type entry points at data, not a directory"));
        }
        for (id, id_off) in p.entries(off & 0x7FFF_FFFF)? {
            if id & 0x8000_0000 != 0 || id > 0xFFFF {
                continue; // a named resource, or an ID MAKEINTRESOURCE cannot express
            }
            if id_off & 0x8000_0000 == 0 {
                return Err(bad("a resource ID entry points at data, not a directory"));
            }
            let langs = p.entries(id_off & 0x7FFF_FFFF)?;
            let Some(&(_, data_entry)) = langs.first() else {
                continue;
            };
            if data_entry & 0x8000_0000 != 0 {
                return Err(bad("a resource language entry is a directory"));
            }
            let e = p
                .rsrc
                .checked_add(data_entry as usize)
                .ok_or_else(|| bad("resource data entry offset overflows"))?;
            let (rva, size) = u32_at(d, e)
                .zip(u32_at(d, e + 4))
                .ok_or_else(|| bad("truncated resource data entry"))?;
            let start = p
                .rva_to_off(rva)
                .ok_or_else(|| bad(format!("resource {id}: its data is outside every section")))?;
            let data = d
                .get(start..start.saturating_add(size as usize))
                .ok_or_else(|| {
                    bad(format!(
                        "resource {id}: its data runs past the end of the file"
                    ))
                })?;
            out.push(Resource {
                id: id as u16,
                data,
            });
        }
    }
    Ok(out)
}
