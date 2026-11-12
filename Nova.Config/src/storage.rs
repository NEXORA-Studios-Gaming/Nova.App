use crate::{ConflictPolicy, Error, Options, Result, error::io};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

fn metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => Ok(Some(meta)),
        Ok(_) => Err(Error::NotRegularFile(path.into())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io(path, error)),
    }
}

pub(crate) fn read(path: &Path, limit: usize) -> Result<Option<Vec<u8>>> {
    let Some(meta) = metadata(path)? else {
        return Ok(None);
    };
    if meta.len() > limit as u64 {
        return Err(Error::TooLarge {
            path: path.into(),
            limit,
        });
    }
    let file = File::open(path).map_err(|e| io(path, e))?;
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| io(path, e))?;
    if bytes.len() > limit {
        return Err(Error::TooLarge {
            path: path.into(),
            limit,
        });
    }
    Ok(Some(bytes))
}

pub(crate) fn save(
    path: &Path,
    bytes: &[u8],
    baseline: Option<&[u8]>,
    options: &Options,
) -> Result<()> {
    if bytes.len() > options.max_file_bytes {
        return Err(Error::TooLarge {
            path: path.into(),
            limit: options.max_file_bytes,
        });
    }
    let parent = path
        .parent()
        .ok_or_else(|| Error::NotRegularFile(path.into()))?;
    if options.create_parent_dirs {
        fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| io(parent, e))?;
    // Existing ordinary permissions survive replacement. ACLs/ownership/xattrs are not copied.
    if let Some(meta) = metadata(path)? {
        temporary
            .as_file()
            .set_permissions(meta.permissions())
            .map_err(|e| io(path, e))?;
    }
    temporary.write_all(bytes).map_err(|e| io(path, e))?;
    if options.sync_on_save {
        temporary.as_file().sync_all().map_err(|e| io(path, e))?;
    }

    // Optimistic check immediately before replacement. This is not a cross-process lock.
    if options.conflict_policy == ConflictPolicy::Reject {
        let current = read(path, options.max_file_bytes)?;
        if current.as_deref() != baseline {
            return Err(Error::Conflict(path.into()));
        }
    } else {
        // Even overwrite mode must not silently replace a directory or a symlink.
        metadata(path)?;
    }
    if baseline.is_none() && options.conflict_policy == ConflictPolicy::Reject {
        // Never replace a file created by someone else during initial creation.
        temporary.persist_noclobber(path).map_err(|e| {
            if e.error.kind() == std::io::ErrorKind::AlreadyExists {
                Error::Conflict(path.into())
            } else {
                io(path, e.error)
            }
        })?;
    } else {
        temporary.persist(path).map_err(|e| io(path, e.error))?;
    }
    Ok(())
}
