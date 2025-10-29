//! Language detection and definitions for 19+ programming languages

use std::collections::HashMap;
use once_cell::sync::Lazy;

#[derive(Debug, Clone)]
pub struct Language {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    pub keywords: &'static [&'static str],
}

/// All supported languages (19+ total)
pub static LANGUAGES: Lazy<Vec<Language>> = Lazy::new(|| vec![
    // Systems Programming
    Language {
        name: "C",
        extensions: &["c", "h"],
        keywords: &["int", "char", "void", "struct", "typedef", "if", "else", "for", "while", "return"],
    },
    Language {
        name: "C++",
        extensions: &["cpp", "hpp", "cc", "cxx", "hxx"],
        keywords: &["class", "namespace", "template", "public", "private", "protected", "virtual"],
    },
    Language {
        name: "Rust",
        extensions: &["rs"],
        keywords: &["fn", "let", "mut", "pub", "impl", "trait", "struct", "enum", "match", "if", "else"],
    },
    Language {
        name: "Go",
        extensions: &["go"],
        keywords: &["func", "package", "import", "var", "const", "type", "struct", "interface", "if", "for"],
    },
    Language {
        name: "Zig",
        extensions: &["zig"],
        keywords: &["fn", "pub", "const", "var", "struct", "enum", "union", "comptime"],
    },
    
    // Web Languages
    Language {
        name: "JavaScript",
        extensions: &["js", "mjs", "cjs"],
        keywords: &["function", "const", "let", "var", "class", "if", "else", "for", "while", "return"],
    },
    Language {
        name: "TypeScript",
        extensions: &["ts", "tsx"],
        keywords: &["interface", "type", "function", "const", "let", "class", "extends", "implements"],
    },
    Language {
        name: "HTML",
        extensions: &["html", "htm"],
        keywords: &["div", "span", "a", "body", "head", "script", "style"],
    },
    Language {
        name: "CSS",
        extensions: &["css"],
        keywords: &["display", "color", "margin", "padding", "border", "background"],
    },
    Language {
        name: "SCSS",
        extensions: &["scss"],
        keywords: &["@mixin", "@include", "@extend", "$variable"],
    },
    Language {
        name: "SASS",
        extensions: &["sass"],
        keywords: &["@mixin", "@include", "@extend", "$variable"],
    },
    Language {
        name: "Less",
        extensions: &["less"],
        keywords: &["@variable", ".mixin"],
    },
    
    // Scripting Languages
    Language {
        name: "Python",
        extensions: &["py", "pyw"],
        keywords: &["def", "class", "if", "else", "elif", "for", "while", "import", "from", "return"],
    },
    Language {
        name: "Ruby",
        extensions: &["rb"],
        keywords: &["def", "class", "module", "if", "else", "elsif", "end", "do", "require"],
    },
    Language {
        name: "Perl",
        extensions: &["pl", "pm"],
        keywords: &["sub", "my", "our", "if", "else", "elsif", "foreach", "while"],
    },
    Language {
        name: "Lua",
        extensions: &["lua"],
        keywords: &["function", "local", "if", "then", "else", "elseif", "end", "for", "while"],
    },
    Language {
        name: "PHP",
        extensions: &["php"],
        keywords: &["function", "class", "if", "else", "elseif", "foreach", "while", "return"],
    },
    
    // JVM Languages
    Language {
        name: "Java",
        extensions: &["java"],
        keywords: &["class", "public", "private", "protected", "void", "if", "else", "for", "while"],
    },
    Language {
        name: "Kotlin",
        extensions: &["kt", "kts"],
        keywords: &["fun", "val", "var", "class", "object", "if", "else", "when", "for"],
    },
    Language {
        name: "Scala",
        extensions: &["scala"],
        keywords: &["def", "val", "var", "class", "object", "trait", "if", "else", "match"],
    },
    Language {
        name: "Groovy",
        extensions: &["groovy"],
        keywords: &["def", "class", "if", "else", "for", "while"],
    },
    Language {
        name: "Clojure",
        extensions: &["clj", "cljs"],
        keywords: &["defn", "def", "let", "if", "cond", "loop", "recur"],
    },
    
    // .NET Languages
    Language {
        name: "C#",
        extensions: &["cs"],
        keywords: &["class", "public", "private", "void", "if", "else", "for", "while", "namespace"],
    },
    Language {
        name: "F#",
        extensions: &["fs", "fsx"],
        keywords: &["let", "type", "module", "if", "then", "else", "match", "with"],
    },
    Language {
        name: "Visual Basic",
        extensions: &["vb"],
        keywords: &["Sub", "Function", "If", "Then", "Else", "For", "While", "End"],
    },
    
    // Functional Languages
    Language {
        name: "Haskell",
        extensions: &["hs"],
        keywords: &["data", "type", "class", "instance", "if", "then", "else", "let", "where"],
    },
    Language {
        name: "OCaml",
        extensions: &["ml", "mli"],
        keywords: &["let", "type", "module", "if", "then", "else", "match", "with"],
    },
    Language {
        name: "Erlang",
        extensions: &["erl"],
        keywords: &["case", "if", "of", "fun", "receive", "after"],
    },
    Language {
        name: "Elixir",
        extensions: &["ex", "exs"],
        keywords: &["defmodule", "def", "defp", "if", "case", "cond", "do", "end"],
    },
    Language {
        name: "Elm",
        extensions: &["elm"],
        keywords: &["type", "alias", "if", "then", "else", "case", "of", "let"],
    },
    Language {
        name: "PureScript",
        extensions: &["purs"],
        keywords: &["data", "type", "class", "instance", "if", "then", "else"],
    },
    Language {
        name: "Reason",
        extensions: &["re"],
        keywords: &["let", "type", "module", "if", "else", "switch"],
    },
    
    // Shell Languages
    Language {
        name: "Bash",
        extensions: &["sh", "bash"],
        keywords: &["if", "then", "else", "fi", "for", "do", "done", "while", "case"],
    },
    Language {
        name: "Zsh",
        extensions: &["zsh"],
        keywords: &["if", "then", "else", "fi", "for", "do", "done", "while"],
    },
    Language {
        name: "Fish",
        extensions: &["fish"],
        keywords: &["function", "if", "else", "end", "for", "while"],
    },
    Language {
        name: "PowerShell",
        extensions: &["ps1"],
        keywords: &["function", "if", "else", "foreach", "while", "$"],
    },
    
    // Data & Config Languages
    Language {
        name: "JSON",
        extensions: &["json"],
        keywords: &[],
    },
    Language {
        name: "YAML",
        extensions: &["yaml", "yml"],
        keywords: &[],
    },
    Language {
        name: "TOML",
        extensions: &["toml"],
        keywords: &[],
    },
    Language {
        name: "XML",
        extensions: &["xml"],
        keywords: &[],
    },
    Language {
        name: "INI",
        extensions: &["ini"],
        keywords: &[],
    },
    
    // Database Languages
    Language {
        name: "SQL",
        extensions: &["sql"],
        keywords: &["SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "ALTER"],
    },
    Language {
        name: "PostgreSQL",
        extensions: &["pgsql"],
        keywords: &["SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE"],
    },
    Language {
        name: "MySQL",
        extensions: &["mysql"],
        keywords: &["SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE"],
    },
    
    // Mobile Development
    Language {
        name: "Swift",
        extensions: &["swift"],
        keywords: &["func", "let", "var", "class", "struct", "enum", "if", "else", "for"],
    },
    Language {
        name: "Objective-C",
        extensions: &["m", "mm"],
        keywords: &["@interface", "@implementation", "@property", "if", "else", "for"],
    },
    Language {
        name: "Dart",
        extensions: &["dart"],
        keywords: &["class", "void", "var", "final", "const", "if", "else", "for"],
    },
    
    // Markup & Documentation
    Language {
        name: "Markdown",
        extensions: &["md", "markdown"],
        keywords: &[],
    },
    Language {
        name: "LaTeX",
        extensions: &["tex"],
        keywords: &["\\documentclass", "\\begin", "\\end", "\\section"],
    },
    Language {
        name: "AsciiDoc",
        extensions: &["adoc", "asciidoc"],
        keywords: &[],
    },
    Language {
        name: "reStructuredText",
        extensions: &["rst"],
        keywords: &[],
    },
    
    // Modern Web Frameworks
    Language {
        name: "Vue",
        extensions: &["vue"],
        keywords: &["template", "script", "style", "export", "default"],
    },
    Language {
        name: "Svelte",
        extensions: &["svelte"],
        keywords: &["script", "style", "export", "let"],
    },
    Language {
        name: "Astro",
        extensions: &["astro"],
        keywords: &["---", "frontmatter"],
    },
    
    // Additional languages (extending to 19+)
    Language { name: "Ada", extensions: &["ada", "adb", "ads"], keywords: &["procedure", "function", "begin", "end"] },
    Language { name: "Assembly", extensions: &["asm", "s"], keywords: &["mov", "push", "pop", "jmp"] },
    Language { name: "AWK", extensions: &["awk"], keywords: &["BEGIN", "END", "if", "else"] },
    Language { name: "Bison", extensions: &["y"], keywords: &["%token", "%type"] },
    Language { name: "Blade", extensions: &["blade.php"], keywords: &["@if", "@foreach", "@extends"] },
    Language { name: "Cairo", extensions: &["cairo"], keywords: &["func", "let", "struct"] },
    Language { name: "CMake", extensions: &["cmake"], keywords: &["add_executable", "project"] },
    Language { name: "COBOL", extensions: &["cob", "cbl"], keywords: &["IDENTIFICATION", "DIVISION"] },
    Language { name: "CoffeeScript", extensions: &["coffee"], keywords: &["->", "=>", "if", "then"] },
    Language { name: "Common Lisp", extensions: &["lisp", "cl"], keywords: &["defun", "lambda", "let"] },
    Language { name: "Crystal", extensions: &["cr"], keywords: &["def", "class", "if", "end"] },
    Language { name: "D", extensions: &["d"], keywords: &["void", "int", "if", "else"] },
    Language { name: "Dockerfile", extensions: &["dockerfile"], keywords: &["FROM", "RUN", "COPY"] },
    Language { name: "Emacs Lisp", extensions: &["el"], keywords: &["defun", "let", "if"] },
    Language { name: "Fortran", extensions: &["f90", "f95"], keywords: &["program", "end", "if"] },
    Language { name: "GDScript", extensions: &["gd"], keywords: &["func", "var", "if", "else"] },
    Language { name: "GLSL", extensions: &["glsl", "vert", "frag"], keywords: &["void", "vec3", "mat4"] },
    Language { name: "GraphQL", extensions: &["graphql", "gql"], keywords: &["query", "mutation", "type"] },
    Language { name: "Hack", extensions: &["hack"], keywords: &["function", "class", "if"] },
    Language { name: "Handlebars", extensions: &["hbs"], keywords: &["{{", "}}", "#if"] },
    Language { name: "Haxe", extensions: &["hx"], keywords: &["function", "class", "var"] },
    Language { name: "HCL", extensions: &["hcl"], keywords: &["resource", "variable"] },
    Language { name: "Janet", extensions: &["janet"], keywords: &["defn", "let", "if"] },
    Language { name: "Julia", extensions: &["jl"], keywords: &["function", "if", "else", "end"] },
    Language { name: "Liquid", extensions: &["liquid"], keywords: &["{%", "%}", "if"] },
    Language { name: "Makefile", extensions: &["makefile", "mk"], keywords: &["all:", "clean:"] },
    Language { name: "MATLAB", extensions: &["m"], keywords: &["function", "if", "end"] },
    Language { name: "Nim", extensions: &["nim"], keywords: &["proc", "var", "let", "if"] },
    Language { name: "Nix", extensions: &["nix"], keywords: &["let", "in", "with"] },
    Language { name: "Objective-C++", extensions: &["mm"], keywords: &["@interface", "class"] },
    Language { name: "Pascal", extensions: &["pas"], keywords: &["program", "begin", "end"] },
    Language { name: "Pug", extensions: &["pug"], keywords: &["div", "span", "if"] },
    Language { name: "R", extensions: &["r"], keywords: &["function", "if", "else"] },
    Language { name: "Racket", extensions: &["rkt"], keywords: &["define", "lambda", "if"] },
    Language { name: "Raku", extensions: &["raku", "p6"], keywords: &["sub", "my", "if"] },
    Language { name: "Scheme", extensions: &["scm"], keywords: &["define", "lambda", "if"] },
    Language { name: "Solidity", extensions: &["sol"], keywords: &["contract", "function", "if"] },
    Language { name: "Starlark", extensions: &["bzl"], keywords: &["def", "if", "else"] },
    Language { name: "Stylus", extensions: &["styl"], keywords: &["color", "margin"] },
    Language { name: "Tcl", extensions: &["tcl"], keywords: &["proc", "if", "else"] },
    Language { name: "Terraform", extensions: &["tf"], keywords: &["resource", "variable"] },
    Language { name: "Vala", extensions: &["vala"], keywords: &["class", "void", "if"] },
    Language { name: "Verilog", extensions: &["v"], keywords: &["module", "input", "output"] },
    Language { name: "VHDL", extensions: &["vhd"], keywords: &["entity", "architecture"] },
    Language { name: "Vim Script", extensions: &["vim"], keywords: &["function", "if", "endif"] },
    Language { name: "WebAssembly", extensions: &["wat"], keywords: &["module", "func"] },
]);

/// Extension to language mapping cache
static EXT_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for lang in LANGUAGES.iter() {
        for ext in lang.extensions {
            map.insert(*ext, lang.name);
        }
    }
    map
});

/// Detect language from file path
pub fn detect_language(path: &str) -> &'static str {
    if let Some(ext) = path.rsplit('.').next() {
        if let Some(&lang) = EXT_MAP.get(ext) {
            return lang;
        }
    }
    "Unknown"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(detect_language("main.rs"), "Rust");
        assert_eq!(detect_language("app.js"), "JavaScript");
        assert_eq!(detect_language("server.py"), "Python");
        assert_eq!(detect_language("Main.java"), "Java");
        assert_eq!(detect_language("unknown.xyz"), "Unknown");
    }

    #[test]
    fn test_all_languages_have_extensions() {
        for lang in LANGUAGES.iter() {
            assert!(!lang.extensions.is_empty(), "{} has no extensions", lang.name);
        }
    }

    #[test]
    fn test_extension_map_size() {
        // Extensions can be shared (e.g., ".m" for both Objective-C and MATLAB)
        // Just verify the map is populated
        assert!(!EXT_MAP.is_empty(), "Extension map should not be empty");
        assert!(EXT_MAP.len() >= 100, "Should have at least 100 unique extensions");
    }
}
