//! Defines object data model used by libalpaca.
use parsing::parse_object_kind;

/// Defines our basic object types, each of which has a corresponding
/// unique (distribution, padding type) tuple.
#[derive(PartialEq)]
pub enum ObjectKind {
    /// Fake "padding" object
    Alpaca,
    /// HTML body
    HTML,
    /// CSS
    CSS,
    /// IMG: PNG, JPEG, etc.
    IMG,
    /// Used when our parser cannot determine the object type
    Unknown,
}

/// An object to be used in the morphing process.
pub struct Object {
    /// Type of the Object
    pub kind: ObjectKind,
    /// Content (Vector of bytes) of the Object
    pub content: Vec<u8>,
    /// Position in the HTML body
    pub position: Option<usize>,
    /// Size to pad the Object to
    pub target_size: Option<usize>,
    /// The uri of the object, as mentioned in the html source (used only for printing warnings)
    pub uri: String,
}

impl Object {
    /// Construct an Object given its content as &str and mime type
    pub fn from_str(cont: &str, mime: &str, uri: String) -> Object {
        Object {
            kind: parse_object_kind(mime),
            content: Vec::from(cont),
            position: None,
            target_size: None,
            uri: uri,
        }
    }

    /// Construct an Object given its content as a byte array and mime type
    pub fn from_raw(raw: &[u8], mime: &str, uri: String) -> Object {
        Object {
            kind: parse_object_kind(mime),
            content: raw.to_vec(),
            position: None,
            target_size: None,
            uri: uri,
        }
    }

    /// Returns a raw pointer to our Object's 'content' field's slice's buffer.
    pub fn as_ptr(self) -> *const u8 {
        let mut buf = self.content.into_boxed_slice();
        let data = buf.as_mut_ptr();
        std::mem::forget(buf);

        data
    }
}
