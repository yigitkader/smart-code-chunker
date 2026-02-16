use tree_sitter::{Language, Node};

pub trait LanguageDriver: Send + Sync {
    fn get_language(&self) -> Language;
    fn get_query(&self) -> &'static str;
    fn get_name(&self) -> &'static str;
    fn extract_name<'a>(&self, node: &Node, content: &'a str) -> Option<&'a str>;
}


struct RustDriver;
impl LanguageDriver for RustDriver {
    fn get_language(&self) -> Language { tree_sitter_rust::language() }
    fn get_name(&self) -> &'static str { "Rust" }
    fn get_query(&self) -> &'static str {
        r#"[ (function_item) (struct_item) (impl_item) (mod_item) ] @chunk"#
    }

    fn extract_name<'a>(&self, node: &Node, content: &'a str) -> Option<&'a str> {
        if let Some(name_node) = node.child_by_field_name("name") {
            return Some(&content[name_node.start_byte()..name_node.end_byte()]);
        }

        if node.kind() == "impl_item" {
            if let Some(type_node) = node.child_by_field_name("type") {
                return Some(&content[type_node.start_byte()..type_node.end_byte()]);
            }
        }
        None
    }
}

struct PythonDriver;
impl LanguageDriver for PythonDriver {
    fn get_language(&self) -> Language { tree_sitter_python::language() }
    fn get_name(&self) -> &'static str { "Python" }
    fn get_query(&self) -> &'static str {
        r#"[ (function_definition) (class_definition) ] @chunk"#
    }

    fn extract_name<'a>(&self, node: &Node, content: &'a str) -> Option<&'a str> {
        if let Some(name_node) = node.child_by_field_name("name") {
            return Some(&content[name_node.start_byte()..name_node.end_byte()]);
        }
        None
    }
}


pub fn get_driver(extension: &str) -> Option<Box<dyn LanguageDriver>> {
    match extension {
        "rs" => Some(Box::new(RustDriver)),
        "py" => Some(Box::new(PythonDriver)),
        _ => None,
    }

}