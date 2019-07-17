//! Contains parsing routines
use std::{str,fs,path::Path};
use select::document::Document;
use select::predicate::Name;

use objects::{Object, ObjectKind};

/// Parses the object's kind from its raw representation
pub fn parse_object_kind(mime: &str) -> ObjectKind {
	match mime {
		"text/html" => ObjectKind::HTML,
		"text/css" => ObjectKind::CSS,
    	"image/png" => ObjectKind::IMG,
    	"image/jpeg" => ObjectKind::IMG,
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
pub fn parse_objects(html: &Object, root: &str, html_path: &str) -> Vec<Object> {
	//Html string
	let html_str = str::from_utf8(&html.content).unwrap();
	//Objects vector
	let mut objects: Vec<Object> = Vec::with_capacity(10);
	let document = Document::from(html_str);

	// Find the css files' paths in the html
    for node in document.find(Name("link")) {
    	match node.attr("rel") {
    		Some(rel) => {
    			if rel == "stylesheet" {
    				match node.attr("href") {
    					Some(path) => {
    						/* Consider the posibility that the css file already has some GET parameters */
    						let split: Vec<&str> = path.split('?').collect();
    						let relative = split[0];
    						
    						let fullpath;
    						match absolute_path(root,relative,html_path) {
    							Some(absolute) => fullpath = absolute,
    							None => continue
    						}

							match fs::read(fullpath) {
			        			Ok(data) => {
			        				let mut object = Object::from_raw(&data,"text/css");
			        				object.position = Some(node.index());
			        				objects.push(object); // Push the new object into the vector
			        			},
			        			Err(_) => continue
			    			}
    					}
    					None => continue
    				}
    			}
    		}
    		None => continue
    	}   	
    }

	// Find the images' paths in the html
    for node in document.find(Name("img")) {
    	match node.attr("src") {
    		Some(path) => {
    			/* Consider the posibility that the image already has some GET parameters */
    			let split: Vec<&str> = path.split('?').collect();
    			let relative = split[0];

		    	let fullpath;
				match absolute_path(root,relative,html_path) {
					Some(absolute) => fullpath = absolute,
					None => continue
				}

				match fs::read(fullpath) {
        			Ok(data) => {
        				let mut object = Object::from_raw(&data,"image/png");
        				object.position = Some(node.index());
        				objects.push(object); // Push the new object into the vector
        			},
        			Err(_) => continue
    			}
    		}
    		None => continue
    	}   	
    }

	objects
}

/// Get the absolute path of a file found in the html.
/// Return None if the file is located in another server
fn absolute_path(root: &str, relative: &str, html_path: &str) -> Option<String> {
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

	absolute.insert_str(0,root); // Make the above path absolute by adding the root

	Some(absolute)
}
