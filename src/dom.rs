//! Contains parsing routines
use std::{str,fs,path::Path};
use kuchiki::traits::*;
use kuchiki::{parse_html_with_options, NodeRef, ParseOpts};
use html5ever::{interface::QualName,LocalName,ns,namespace_url,serialize,serialize::{SerializeOpts}};

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
    /// Node in the html
    pub node: Option<NodeRef>,
    /// Size to pad the Object to
    pub target_size: Option<usize>,
    /// The uri of the object, as mentioned in the html source
    pub uri: String,
}

impl Object {
    /// Construct a real object from the html page
    pub fn real(content: &[u8], mime: &str, uri: String, node: &NodeRef) -> Object {
        Object {
            kind: parse_object_kind(mime),
            content: content.to_vec(),
            node: Some(node.clone()),
            target_size: None,
            uri: uri,
        }
    }

    /// Create padding object
    pub fn padding(target_size: usize) -> Object {
        Object {
            kind: ObjectKind::Alpaca,
            content: Vec::new(),
            node: None,
            target_size: Some(target_size),
            uri: String::from("pad_object"),
        }
    }
}




/// Parses the object's kind from its raw representation
pub fn parse_object_kind(mime: &str) -> ObjectKind {
	match mime {
		"text/html" => ObjectKind::HTML,
		"text/css" => ObjectKind::CSS,
		x if x.starts_with("image/") => ObjectKind::IMG,
    	_=> ObjectKind::Unknown
    }
}

/// Parses the target size of an object from its HTTP request query.
/// Returns 0 on error.
pub fn parse_target_size(query: &str) -> usize {
	let split1: Vec<&str> = query.split("alpaca-padding=").collect();
	let split2: Vec<&str> = split1[split1.len()-1].split("&").collect();
	let size_str = split2[0];

	//Return the size
	match size_str.parse::<usize>() {
	  Ok(size) => return size,
	  Err(_) => return 0
	}
}

/// Parses the objects contained in an HTML page.
//
pub fn parse_objects(document: &NodeRef, root: &str, html_path: &str, alias: usize) -> Vec<Object> {
	//Objects vector
	let mut objects: Vec<Object> = Vec::with_capacity(10);
	let mut found_favicon = false;

	// Find the css files' paths in the html
    for node_data in document.select("link").unwrap() {
		let node = node_data.as_node();
    	match (node_get_attribute(node, "rel"), node_get_attribute(node, "href")) {
    		(Some(rel), Some(path)) if rel == "stylesheet" => {
				/* Consider the posibility that the css file already has some GET parameters */
				let split: Vec<&str> = path.split('?').collect();
				let relative = split[0];
				
				let fullpath;
				match uri_to_abs_fs_path(root,relative,html_path,alias) {
					Some(absolute) => fullpath = absolute,
					None => continue
				}

				match fs::read(fullpath) {
					Ok(data) => objects.push(Object::real(&data,"text/css", path, node)),
					Err(_) => continue,
				}
			},
    		_ => continue
    	}   	
    }

	// Find the images' paths in the html (<img> tags but also <link href="favicon.ico" rel="shortcut icon">)
    for node_data in document.select("img,link").unwrap() {
		let node = node_data.as_node();

		let mut path_attr = "src";
		if node_data.name.local.to_lowercase() == "link" {
			match node_get_attribute(node, "rel").unwrap_or_default().as_ref() {
				"shortcut icon" | "icon" => {
					found_favicon = true;
					path_attr = "href";
				},
				_ => continue,
			}
		}

    	match node_get_attribute(node, path_attr) {
    		Some(path) => {
    			/* Consider the posibility that the image already has some GET parameters */
    			let split: Vec<&str> = path.split('?').collect();
    			let relative = split[0];

		    	let fullpath;
				match uri_to_abs_fs_path(root,relative,html_path,alias) {
					Some(absolute) => fullpath = absolute,
					None => continue
				}

				match fs::read(fullpath) {
        			Ok(data) => objects.push(Object::real(&data, "image/any", path, node)),
        			Err(_) => continue,
    			}
    		}
    		None => continue
    	}   	
    }

	// If no favicon was found, insert an empty one
	if !found_favicon {
		insert_empty_favicon(document);
	}

    objects.sort_unstable_by(|a, b| b.content.len().cmp(&a.content.len()));		// larger first
	objects
}

pub fn insert_empty_favicon(document: &NodeRef) {
    // append the <link> either to the <head> tag, if exists, otherwise
    // to the whole document
    let node_data;  // to outlive the match
    let node = match document.select("head").unwrap().next() {
        Some(nd) => { node_data = nd; node_data.as_node() },
        None => document,
    };

	let elem = create_element("link");
	node_set_attribute(&elem, "href", String::from("#"));
	node_set_attribute(&elem, "rel", String::from("shortcut icon"));
	node.append(elem);
}

/// Maps a (relative or absolute) uri, to an absolute filesystem path.
/// Returns None if uri_path is located in another server
fn uri_to_abs_fs_path(root: &str, relative: &str, html_path: &str, alias: usize) -> Option<String> {
	if relative.starts_with("https://") || relative.starts_with("http://") {
		return None;
	}

	let mut fs_relative = String::from(relative);

	if !fs_relative.starts_with('/') {
		let base = Path::new(html_path).parent().unwrap().to_str().unwrap();
		
		if !base.ends_with('/') {
			fs_relative.insert(0,'/');
		}
		fs_relative.insert_str(0,base);
	}

	// Resolve the dots in the path so far
	let components: Vec<&str> = fs_relative.split("/").collect(); 	// Original components of the path

	let mut normalized: Vec<String> = Vec::with_capacity(components.len()); // Stack to be used for the normalization	

	for comp in components {
		if comp == "." || comp == "" {continue;}
		else if comp == ".." {
			if !normalized.is_empty() {
				normalized.pop();
			}
		}
		else {
			normalized.push("/".to_string()+comp);
		}
	}

	let mut absolute: String = normalized.into_iter().collect(); // String with the resolved relative path

	if html_path[..alias] != absolute[..alias] {
		return None;
	}

	absolute = absolute[alias..].to_string(); // Remove alias characters in case there are any

	absolute.insert_str(0,root); // Make the above path absolute by adding the root

	Some(absolute)
}

pub fn parse_html(input: &str) -> NodeRef {
    let mut opts = ParseOpts::default();
    opts.tree_builder.drop_doctype = true;

    let mut parser = parse_html_with_options(opts);
    parser.process(input.into());
    parser.finish()
}

pub fn serialize_html(dom: &NodeRef) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    let opts = SerializeOpts::default();

    serialize(&mut buf, dom, opts)
        .expect("serialization failed");

    buf
}

pub fn create_element(name: &str) -> NodeRef {
    let qual_name = QualName::new(None, ns!(), LocalName::from(name));
    NodeRef::new_element(qual_name, Vec::new())
}

fn node_get_attribute(node: &NodeRef, name: &str) -> Option<String> {
    match node.as_element() {
        Some(element) => {
            match element.attributes.borrow().get(name) {
                Some(val) => Some(String::from(val)),
                None => None,
            }
        },
        None => None,
    }
}

pub fn node_set_attribute(node: &NodeRef, name: &str, value: String) {
    let elem = node.as_element().unwrap();
    elem.attributes.borrow_mut().insert(name, value);
}