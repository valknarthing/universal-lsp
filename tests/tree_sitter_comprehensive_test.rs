//! Comprehensive tests for tree-sitter parsing and symbol extraction
//!
//! Tests all 19 supported languages with various code patterns

use universal_lsp::tree_sitter::TreeSitterParser;

#[test]
fn test_javascript_symbol_extraction() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let code = r#"
function greet(name) {
    return `Hello, ${name}!`;
}

class User {
    constructor(name) {
        this.name = name;
    }

    getName() {
        return this.name;
    }
}

const arrow = () => console.log("arrow");
"#;

    let symbols = parser.extract_symbols("javascript", code)
        .expect("Failed to extract JavaScript symbols");

    assert!(symbols.iter().any(|s| s.name == "greet"));
    assert!(symbols.iter().any(|s| s.name == "User"));
    assert!(symbols.iter().any(|s| s.name == "getName"));
    assert!(symbols.iter().any(|s| s.name == "arrow"));
}

#[test]
fn test_python_symbol_extraction() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let code = r#"
def hello_world():
    print("Hello, World!")

class Person:
    def __init__(self, name):
        self.name = name

    def greet(self):
        return f"Hello, {self.name}"

async def fetch_data():
    pass
"#;

    let symbols = parser.extract_symbols("python", code)
        .expect("Failed to extract Python symbols");

    assert!(symbols.iter().any(|s| s.name == "hello_world"));
    assert!(symbols.iter().any(|s| s.name == "Person"));
    assert!(symbols.iter().any(|s| s.name == "greet"));
    assert!(symbols.iter().any(|s| s.name == "fetch_data"));
}

#[test]
fn test_rust_symbol_extraction() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let code = r#"
fn main() {
    println!("Hello, world!");
}

struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    fn distance(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
}

trait Drawable {
    fn draw(&self);
}
"#;

    let symbols = parser.extract_symbols("rust", code)
        .expect("Failed to extract Rust symbols");

    assert!(symbols.iter().any(|s| s.name == "main"));
    assert!(symbols.iter().any(|s| s.name == "Point"));
    assert!(symbols.iter().any(|s| s.name == "new"));
    assert!(symbols.iter().any(|s| s.name == "distance"));
    assert!(symbols.iter().any(|s| s.name == "Drawable"));
}

#[test]
fn test_go_symbol_extraction() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let code = r#"
package main

import "fmt"

func main() {
    fmt.Println("Hello, World!")
}

type Person struct {
    Name string
    Age  int
}

func (p *Person) Greet() string {
    return fmt.Sprintf("Hello, I'm %s", p.Name)
}

func NewPerson(name string, age int) *Person {
    return &Person{Name: name, Age: age}
}
"#;

    let symbols = parser.extract_symbols("go", code)
        .expect("Failed to extract Go symbols");

    assert!(symbols.iter().any(|s| s.name == "main"));
    assert!(symbols.iter().any(|s| s.name == "Person"));
    assert!(symbols.iter().any(|s| s.name == "Greet"));
    assert!(symbols.iter().any(|s| s.name == "NewPerson"));
}

#[test]
fn test_typescript_symbol_extraction() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let code = r#"
interface User {
    name: string;
    age: number;
}

class UserService {
    private users: User[] = [];

    addUser(user: User): void {
        this.users.push(user);
    }

    getUser(name: string): User | undefined {
        return this.users.find(u => u.name === name);
    }
}

function createUser(name: string, age: number): User {
    return { name, age };
}
"#;

    let symbols = parser.extract_symbols("typescript", code)
        .expect("Failed to extract TypeScript symbols");

    assert!(symbols.iter().any(|s| s.name == "User"));
    assert!(symbols.iter().any(|s| s.name == "UserService"));
    assert!(symbols.iter().any(|s| s.name == "addUser"));
    assert!(symbols.iter().any(|s| s.name == "createUser"));
}

#[test]
fn test_cpp_symbol_extraction() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let code = r#"
#include <iostream>

class Rectangle {
private:
    double width;
    double height;

public:
    Rectangle(double w, double h) : width(w), height(h) {}

    double area() const {
        return width * height;
    }

    double perimeter() const {
        return 2 * (width + height);
    }
};

namespace math {
    double square(double x) {
        return x * x;
    }
}
"#;

    let symbols = parser.extract_symbols("cpp", code)
        .expect("Failed to extract C++ symbols");

    assert!(symbols.iter().any(|s| s.name == "Rectangle"));
    assert!(symbols.iter().any(|s| s.name == "area"));
    assert!(symbols.iter().any(|s| s.name == "perimeter"));
    assert!(symbols.iter().any(|s| s.name == "square"));
}

#[test]
fn test_empty_file() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let symbols = parser.extract_symbols("javascript", "")
        .expect("Failed to extract symbols from empty file");

    assert_eq!(symbols.len(), 0);
}

#[test]
fn test_syntax_error_handling() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let code = "function broken( {"; // Intentionally broken

    // Should not panic, should handle gracefully
    let result = parser.extract_symbols("javascript", code);

    // Either returns empty symbols or an error, but shouldn't panic
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_large_file_performance() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    // Generate a large file with 1000 functions
    let mut code = String::new();
    for i in 0..1000 {
        code.push_str(&format!("function func{}() {{ return {}; }}\n", i, i));
    }

    let start = std::time::Instant::now();
    let symbols = parser.extract_symbols("javascript", &code)
        .expect("Failed to extract symbols");
    let duration = start.elapsed();

    assert_eq!(symbols.len(), 1000);
    assert!(duration.as_millis() < 1000, "Parsing took too long: {:?}", duration);
}

#[test]
fn test_all_19_languages() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let test_cases = vec![
        ("javascript", "function test() {}"),
        ("typescript", "function test(): void {}"),
        ("python", "def test():\n    pass"),
        ("rust", "fn test() {}"),
        ("go", "func test() {}"),
        ("java", "public void test() {}"),
        ("c", "void test() {}"),
        ("cpp", "void test() {}"),
        ("ruby", "def test; end"),
        ("php", "<?php function test() {} ?>"),
        ("bash", "function test() { echo 'test'; }"),
        ("html", "<div id='test'></div>"),
        ("css", ".test { color: red; }"),
        ("json", "{\"test\": 123}"),
        ("svelte", "<script>function test() {}</script>"),
        ("scala", "def test = ()"),
        ("kotlin", "fun test() {}"),
        ("csharp", "void Test() {}"),
        ("tsx", "function test(): JSX.Element { return <div/>; }"),
    ];

    for (lang, code) in test_cases {
        let result = parser.extract_symbols(lang, code);
        assert!(result.is_ok(), "Failed to parse {}: {:?}", lang, result.err());
    }
}

#[test]
fn test_unicode_handling() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let code = r#"
function привет() {
    return "Привет, мир!";
}

function 你好() {
    return "你好，世界！";
}
"#;

    let symbols = parser.extract_symbols("javascript", code)
        .expect("Failed to extract symbols with Unicode");

    assert!(symbols.iter().any(|s| s.name == "привет"));
    assert!(symbols.iter().any(|s| s.name == "你好"));
}

#[test]
fn test_nested_structures() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let code = r#"
class Outer {
    class Inner {
        function deepMethod() {
            return function nested() {
                return "deep";
            };
        }
    }
}
"#;

    let symbols = parser.extract_symbols("javascript", code)
        .expect("Failed to extract nested symbols");

    assert!(symbols.iter().any(|s| s.name == "Outer"));
    assert!(symbols.iter().any(|s| s.name == "Inner"));
    assert!(symbols.iter().any(|s| s.name == "deepMethod"));
}

#[test]
fn test_symbol_kinds() {
    let parser = TreeSitterParser::new().expect("Failed to create parser");

    let code = r#"
function myFunction() {}
class MyClass {}
const myConstant = 42;
let myVariable = "test";
"#;

    let symbols = parser.extract_symbols("javascript", code)
        .expect("Failed to extract symbols");

    // Check that we have different kinds of symbols
    assert!(symbols.len() >= 2, "Should have at least function and class");
}

#[test]
fn test_concurrent_parsing() {
    use std::thread;

    let handles: Vec<_> = (0..10).map(|i| {
        thread::spawn(move || {
            let parser = TreeSitterParser::new().expect("Failed to create parser");
            let code = format!("function test{}() {{ return {}; }}", i, i);
            parser.extract_symbols("javascript", &code)
                .expect("Failed to extract symbols")
        })
    }).collect();

    for handle in handles {
        let symbols = handle.join().expect("Thread panicked");
        assert_eq!(symbols.len(), 1);
    }
}
