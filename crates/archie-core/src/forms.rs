//! The forms a document takes outside archie, markdown, JSON and rkyv, each
//! a pair of functions from bytes to a document and back.

use core::str::from_utf8;

use rkyv::de::Pool;
use rkyv::rancor::{self, Strategy};
use rkyv::util::AlignedVec;
use rkyv::with::{DeserializeWith as _, With};

use crate::parser::ast::{ArchivedRawDocument, RawDocument};
use crate::parser::mirror::RawDocumentMirror;
use crate::parser::{from_json, parse, to_json};
use crate::writer::{self, WriteError, WriteOptions, write_archived};

/// `archie` and a zero byte, the bytes the rkyv form opens with.
pub const RKYV_MAGIC: &[u8; 7] = b"archie\0";

/// The version of the model's types, the byte after `RKYV_MAGIC`.
pub const RKYV_VERSION: u8 = 3;

/// A form a document takes outside archie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Form {
    Json,
    Markdown,
    Rkyv,
}

/// Why bytes do not hold a document in a form.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DecodeError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("not archie's rkyv form: the bytes do not open with `archie` and a zero byte")]
    NotArchie,
    #[error("not UTF-8 at byte {offset}")]
    NotUtf8 { offset: usize },
    #[error(transparent)]
    Rkyv(#[from] rancor::Error),
    #[error("archie's rkyv form version {found}, and this archie reads version {RKYV_VERSION}")]
    Version { found: u8 },
}

/// Why a document could not be put into a form.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EncodeError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Rkyv(#[from] rancor::Error),
    #[error(transparent)]
    Write(#[from] WriteError),
}

/// Why the text a store keeps could not be made from bytes in a form.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FormError {
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error(transparent)]
    Encode(#[from] EncodeError),
}

/// The bytes of `document` in `form`. `options` names the frontmatter syntax
/// of the markdown form.
///
/// # Errors
///
/// The writer's error for the markdown form, `serde_json`'s or rkyv's for
/// the others.
pub fn encode(
    document: &RawDocument,
    form: Form,
    options: &WriteOptions,
) -> Result<Vec<u8>, EncodeError> {
    match form {
        Form::Json => {
            let mut text = to_json(document)?;
            text.push('\n');
            Ok(text.into_bytes())
        }
        Form::Markdown => Ok(writer::write(document, options)?.into_bytes()),
        Form::Rkyv => {
            let archived = rkyv::to_bytes::<rancor::Error>(
                With::<RawDocument, RawDocumentMirror>::cast(document),
            )?;
            Ok([RKYV_MAGIC.as_slice(), &[RKYV_VERSION], &archived].concat())
        }
    }
}

/// The document `bytes` hold in `form`. The rkyv form is deserialized here; a
/// caller that only writes it reads it in place through `stored_text`.
///
/// # Errors
///
/// `DecodeError::NotUtf8` naming the offset of the first byte that is not, a
/// JSON error, a header that is not archie's or names another version, an
/// rkyv error.
pub fn decode(bytes: &[u8], form: Form) -> Result<RawDocument, DecodeError> {
    match form {
        Form::Json => Ok(from_json(text(bytes)?)?),
        Form::Markdown => Ok(parse(text(bytes)?).document),
        Form::Rkyv => {
            let buffer = aligned(bytes)?;
            let archived = rkyv::access::<ArchivedRawDocument, rancor::Error>(&buffer)?;
            Ok(RawDocumentMirror::deserialize_with(
                archived,
                Strategy::<_, rancor::Error>::wrap(&mut Pool::new()),
            )?)
        }
    }
}

/// The text a store keeps for `bytes` in `form`: archie's form, its
/// frontmatter in the syntax `options` names. JSON and rkyv are written, read
/// and written again; rkyv is read in place with `write_archived`.
///
/// # Errors
///
/// The errors of `decode` and `encode`.
pub fn stored_text(bytes: &[u8], form: Form, options: &WriteOptions) -> Result<String, FormError> {
    let first = match form {
        Form::Json => writer::write(&decode(bytes, form)?, options),
        Form::Markdown => return Ok(rewritten(text(bytes)?, options)?),
        Form::Rkyv => {
            let buffer = aligned(bytes)?;
            let archived = rkyv::access::<ArchivedRawDocument, rancor::Error>(&buffer)
                .map_err(DecodeError::from)?;
            write_archived(archived, options)
        }
    };
    Ok(rewritten(&first.map_err(EncodeError::from)?, options)?)
}

/// `bytes` as text, or the offset of the first byte that is not UTF-8.
fn text(bytes: &[u8]) -> Result<&str, DecodeError> {
    from_utf8(bytes).map_err(|error| DecodeError::NotUtf8 {
        offset: error.valid_up_to(),
    })
}

/// The bytes after archie's rkyv header, copied where rkyv reads them in place.
fn aligned(bytes: &[u8]) -> Result<AlignedVec, DecodeError> {
    let (&version, archived) = bytes
        .strip_prefix(RKYV_MAGIC.as_slice())
        .and_then(<[u8]>::split_first)
        .ok_or(DecodeError::NotArchie)?;
    if version != RKYV_VERSION {
        return Err(DecodeError::Version { found: version });
    }
    let mut buffer = AlignedVec::with_capacity(archived.len());
    buffer.extend_from_slice(archived);
    Ok(buffer)
}

/// `text` parsed and written in archie's form.
fn rewritten(text: &str, options: &WriteOptions) -> Result<String, EncodeError> {
    Ok(writer::write(&parse(text).document, options)?)
}

#[cfg(test)]
mod tests;
