#!/bin/bash
#
# Universal LSP Development Environment Setup
# Interactive script to configure IDE, AI assistant, and project stack
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration variables
IDE=""
AI_PROVIDER=""
PROJECT_TYPE=""
PROJECT_NAME=""
USE_WORKSPACE_CONFIG=""
CONFIG_DIR=""

#==============================================================================
# Helper Functions
#==============================================================================

print_header() {
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║           Universal LSP Development Environment Setup                ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

print_section() {
    echo -e "\n${MAGENTA}═══ $1 ═══${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

prompt_choice() {
    local prompt="$1"
    local default="$2"
    local response

    echo -e "${CYAN}$prompt${NC}"
    if [ -n "$default" ]; then
        echo -e "${YELLOW}[Default: $default]${NC}"
    fi
    read -p "> " response

    if [ -z "$response" ] && [ -n "$default" ]; then
        response="$default"
    fi

    echo "$response"
}

#==============================================================================
# IDE Selection
#==============================================================================

select_ide() {
    print_section "IDE Selection"

    echo "Please select your IDE:"
    echo "  1) Zed (Recommended - Fast, Rust-based)"
    echo "  2) VSCode (Full-featured)"
    echo "  3) Neovim (Terminal-based)"
    echo "  4) Other (Manual configuration)"
    echo ""

    local choice=$(prompt_choice "Enter your choice [1-4]:" "1")

    case $choice in
        1) IDE="zed" ;;
        2) IDE="vscode" ;;
        3) IDE="neovim" ;;
        4) IDE="other" ;;
        *)
            print_error "Invalid choice. Using Zed as default."
            IDE="zed"
            ;;
    esac

    print_success "Selected IDE: $IDE"
}

#==============================================================================
# AI Provider Selection
#==============================================================================

select_ai_provider() {
    print_section "AI Assistant Configuration"

    echo "Please select your AI coding assistant:"
    echo "  1) Claude (Anthropic)"
    echo "  2) GitHub Copilot"
    echo "  3) Both (Claude + Copilot)"
    echo "  4) None"
    echo ""

    local choice=$(prompt_choice "Enter your choice [1-4]:" "1")

    case $choice in
        1) AI_PROVIDER="claude" ;;
        2) AI_PROVIDER="copilot" ;;
        3) AI_PROVIDER="both" ;;
        4) AI_PROVIDER="none" ;;
        *)
            print_error "Invalid choice. Using Claude as default."
            AI_PROVIDER="claude"
            ;;
    esac

    print_success "Selected AI provider: $AI_PROVIDER"
}

#==============================================================================
# Project Type Selection
#==============================================================================

select_project_type() {
    print_section "Project Stack Selection"

    echo "Please select your project type:"
    echo "  1) Svelte 5 + Tailwind 4 (Modern SPA)"
    echo "  2) React + TypeScript"
    echo "  3) Vue 3 + Vite"
    echo "  4) Next.js (React SSR)"
    echo "  5) Python (FastAPI/Django)"
    echo "  6) Rust (CLI/Web)"
    echo "  7) Go (Backend)"
    echo "  8) Full-stack (Node.js + TypeScript)"
    echo "  9) Custom (Manual configuration)"
    echo ""

    local choice=$(prompt_choice "Enter your choice [1-9]:" "1")

    case $choice in
        1) PROJECT_TYPE="svelte-tailwind" ;;
        2) PROJECT_TYPE="react-ts" ;;
        3) PROJECT_TYPE="vue-vite" ;;
        4) PROJECT_TYPE="nextjs" ;;
        5) PROJECT_TYPE="python" ;;
        6) PROJECT_TYPE="rust" ;;
        7) PROJECT_TYPE="go" ;;
        8) PROJECT_TYPE="fullstack-node" ;;
        9) PROJECT_TYPE="custom" ;;
        *)
            print_error "Invalid choice. Using Svelte + Tailwind as default."
            PROJECT_TYPE="svelte-tailwind"
            ;;
    esac

    print_success "Selected project type: $PROJECT_TYPE"
}

#==============================================================================
# Project Name Input
#==============================================================================

get_project_name() {
    print_section "Project Configuration"

    PROJECT_NAME=$(prompt_choice "Enter project name:" "my-project")
    PROJECT_NAME=$(echo "$PROJECT_NAME" | tr '[:upper:]' '[:lower:]' | tr ' ' '-')

    print_success "Project name: $PROJECT_NAME"
}

#==============================================================================
# Workspace Configuration
#==============================================================================

configure_workspace() {
    print_section "Workspace Configuration"

    echo "Do you want to create a multi-root workspace configuration?"
    echo "  y) Yes - Multiple project folders with shared settings"
    echo "  n) No - Single project configuration"
    echo ""

    local choice=$(prompt_choice "Your choice [y/n]:" "n")

    case $choice in
        [Yy]*) USE_WORKSPACE_CONFIG="yes" ;;
        *) USE_WORKSPACE_CONFIG="no" ;;
    esac

    print_success "Multi-root workspace: $USE_WORKSPACE_CONFIG"
}

#==============================================================================
# Zed Configuration
#==============================================================================

setup_zed() {
    print_section "Configuring Zed"

    CONFIG_DIR="$HOME/.config/zed"
    mkdir -p "$CONFIG_DIR"

    # Create settings.json
    cat > "$CONFIG_DIR/settings.json" <<EOF
{
  "theme": "One Dark",
  "buffer_font_size": 14,
  "ui_font_size": 14,
  "tab_size": 2,
  "soft_wrap": "editor_width",
  "format_on_save": "on",
  "languages": {
    "JavaScript": {
      "tab_size": 2,
      "formatter": {
        "external": {
          "command": "prettier",
          "arguments": ["--stdin-filepath", "{buffer_path}"]
        }
      }
    },
    "TypeScript": {
      "tab_size": 2,
      "formatter": {
        "external": {
          "command": "prettier",
          "arguments": ["--stdin-filepath", "{buffer_path}"]
        }
      }
    },
    "Rust": {
      "tab_size": 4,
      "formatter": "language_server"
    }
  },
  "lsp": {
    "universal-lsp": {
      "binary": {
        "path": "$PWD/target/release/universal-lsp"
      }
    }
  }
}
EOF

    # Add AI provider configuration
    if [ "$AI_PROVIDER" = "claude" ] || [ "$AI_PROVIDER" = "both" ]; then
        print_info "Configuring Claude AI integration..."
        # Add Claude configuration to settings
    fi

    if [ "$AI_PROVIDER" = "copilot" ] || [ "$AI_PROVIDER" = "both" ]; then
        print_info "Configuring GitHub Copilot..."
        # Add Copilot configuration
    fi

    print_success "Zed configuration created at $CONFIG_DIR/settings.json"
}

#==============================================================================
# VSCode Configuration
#==============================================================================

setup_vscode() {
    print_section "Configuring VSCode"

    CONFIG_DIR=".vscode"
    mkdir -p "$CONFIG_DIR"

    # Create settings.json
    cat > "$CONFIG_DIR/settings.json" <<EOF
{
  "editor.fontSize": 14,
  "editor.tabSize": 2,
  "editor.formatOnSave": true,
  "editor.minimap.enabled": false,
  "workbench.colorTheme": "One Dark Pro",
  "files.autoSave": "onFocusChange",
  "javascript.updateImportsOnFileMove.enabled": "always",
  "typescript.updateImportsOnFileMove.enabled": "always"
}
EOF

    # Create extensions.json
    local extensions='["dbaeumer.vscode-eslint", "esbenp.prettier-vscode"'

    if [ "$AI_PROVIDER" = "claude" ] || [ "$AI_PROVIDER" = "both" ]; then
        extensions+=', "anthropics.claude-code"'
    fi

    if [ "$AI_PROVIDER" = "copilot" ] || [ "$AI_PROVIDER" = "both" ]; then
        extensions+=', "github.copilot", "github.copilot-chat"'
    fi

    if [ "$PROJECT_TYPE" = "svelte-tailwind" ]; then
        extensions+=', "svelte.svelte-vscode", "bradlc.vscode-tailwindcss"'
    fi

    extensions+=']'

    cat > "$CONFIG_DIR/extensions.json" <<EOF
{
  "recommendations": $extensions
}
EOF

    print_success "VSCode configuration created in $CONFIG_DIR/"
}

#==============================================================================
# Workspace Configuration File
#==============================================================================

create_workspace_config() {
    print_section "Creating Workspace Configuration"

    local config_file=".universal-lsp.toml"

    cat > "$config_file" <<EOF
# Universal LSP Workspace Configuration
# Generated by dev-env-setup.sh

[workspace]
name = "$PROJECT_NAME"
root = "."

[formatting]
indent_size = 2
use_tabs = false
max_line_length = 100

[languages.javascript]
indent_size = 2
formatter = "prettier"

[languages.typescript]
indent_size = 2
formatter = "prettier"

[languages.rust]
indent_size = 4
formatter = "rustfmt"

[languages.python]
indent_size = 4
formatter = "black"

[excluded_paths]
patterns = [
  "**/node_modules/**",
  "**/target/**",
  "**/.git/**",
  "**/dist/**",
  "**/build/**"
]
EOF

    print_success "Workspace configuration created: $config_file"
}

#==============================================================================
# Project Scaffolding
#==============================================================================

scaffold_project() {
    print_section "Scaffolding Project: $PROJECT_TYPE"

    case $PROJECT_TYPE in
        "svelte-tailwind")
            scaffold_svelte_tailwind
            ;;
        "react-ts")
            scaffold_react_ts
            ;;
        "vue-vite")
            scaffold_vue_vite
            ;;
        "nextjs")
            scaffold_nextjs
            ;;
        "python")
            scaffold_python
            ;;
        "rust")
            scaffold_rust
            ;;
        "go")
            scaffold_go
            ;;
        "fullstack-node")
            scaffold_fullstack_node
            ;;
        "custom")
            print_info "Skipping project scaffolding for custom configuration"
            ;;
    esac
}

scaffold_svelte_tailwind() {
    print_info "Creating Svelte 5 + Tailwind 4 project..."

    if command -v npm &> /dev/null; then
        npm create vite@latest "$PROJECT_NAME" -- --template svelte-ts
        cd "$PROJECT_NAME"
        npm install -D tailwindcss@next postcss autoprefixer
        npx tailwindcss init -p

        # Create Tailwind config for v4
        cat > tailwind.config.js <<EOF
/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {},
  },
  plugins: [],
}
EOF

        # Add Tailwind directives to app.css
        cat > src/app.css <<EOF
@tailwind base;
@tailwind components;
@tailwind utilities;
EOF

        cd ..
        print_success "Svelte + Tailwind project created successfully"
    else
        print_error "npm not found. Please install Node.js first."
    fi
}

scaffold_react_ts() {
    print_info "Creating React + TypeScript project..."

    if command -v npm &> /dev/null; then
        npm create vite@latest "$PROJECT_NAME" -- --template react-ts
        print_success "React + TypeScript project created successfully"
    else
        print_error "npm not found. Please install Node.js first."
    fi
}

scaffold_vue_vite() {
    print_info "Creating Vue 3 + Vite project..."

    if command -v npm &> /dev/null; then
        npm create vite@latest "$PROJECT_NAME" -- --template vue-ts
        print_success "Vue 3 + Vite project created successfully"
    else
        print_error "npm not found. Please install Node.js first."
    fi
}

scaffold_nextjs() {
    print_info "Creating Next.js project..."

    if command -v npx &> /dev/null; then
        npx create-next-app@latest "$PROJECT_NAME" --typescript --tailwind --app --no-src-dir
        print_success "Next.js project created successfully"
    else
        print_error "npm/npx not found. Please install Node.js first."
    fi
}

scaffold_python() {
    print_info "Creating Python project..."

    mkdir -p "$PROJECT_NAME"
    cd "$PROJECT_NAME"

    # Create virtual environment
    python3 -m venv venv

    # Create requirements.txt
    cat > requirements.txt <<EOF
fastapi==0.104.1
uvicorn[standard]==0.24.0
pydantic==2.5.0
python-multipart==0.0.6
EOF

    # Create main.py
    cat > main.py <<EOF
from fastapi import FastAPI

app = FastAPI()

@app.get("/")
async def root():
    return {"message": "Hello World"}

@app.get("/health")
async def health():
    return {"status": "healthy"}
EOF

    # Create .gitignore
    cat > .gitignore <<EOF
venv/
__pycache__/
*.pyc
.env
.pytest_cache/
EOF

    cd ..
    print_success "Python project created successfully"
}

scaffold_rust() {
    print_info "Creating Rust project..."

    if command -v cargo &> /dev/null; then
        cargo new "$PROJECT_NAME"
        print_success "Rust project created successfully"
    else
        print_error "cargo not found. Please install Rust first."
    fi
}

scaffold_go() {
    print_info "Creating Go project..."

    if command -v go &> /dev/null; then
        mkdir -p "$PROJECT_NAME"
        cd "$PROJECT_NAME"
        go mod init "$PROJECT_NAME"

        cat > main.go <<EOF
package main

import (
    "fmt"
    "net/http"
)

func main() {
    http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
        fmt.Fprintf(w, "Hello, World!")
    })

    fmt.Println("Server starting on :8080")
    http.ListenAndServe(":8080", nil)
}
EOF

        cd ..
        print_success "Go project created successfully"
    else
        print_error "go not found. Please install Go first."
    fi
}

scaffold_fullstack_node() {
    print_info "Creating Full-stack Node.js project..."

    mkdir -p "$PROJECT_NAME"
    cd "$PROJECT_NAME"

    npm init -y
    npm install express typescript @types/node @types/express ts-node
    npm install -D nodemon

    # Create tsconfig.json
    cat > tsconfig.json <<EOF
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "lib": ["ES2020"],
    "outDir": "./dist",
    "rootDir": "./src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true
  },
  "include": ["src/**/*"],
  "exclude": ["node_modules"]
}
EOF

    mkdir -p src
    cat > src/index.ts <<EOF
import express from 'express';

const app = express();
const PORT = process.env.PORT || 3000;

app.use(express.json());

app.get('/', (req, res) => {
  res.json({ message: 'Hello World' });
});

app.listen(PORT, () => {
  console.log(\`Server running on port \${PORT}\`);
});
EOF

    cd ..
    print_success "Full-stack Node.js project created successfully"
}

#==============================================================================
# Summary and Next Steps
#==============================================================================

print_summary() {
    print_section "Setup Complete!"

    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                        Configuration Summary                         ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  ${CYAN}IDE:${NC}               $IDE"
    echo -e "  ${CYAN}AI Provider:${NC}       $AI_PROVIDER"
    echo -e "  ${CYAN}Project Type:${NC}      $PROJECT_TYPE"
    echo -e "  ${CYAN}Project Name:${NC}      $PROJECT_NAME"
    echo -e "  ${CYAN}Multi-root WS:${NC}     $USE_WORKSPACE_CONFIG"
    echo ""

    print_section "Next Steps"

    echo "1. Build the Universal LSP server:"
    echo -e "   ${YELLOW}cd $(pwd) && cargo build --release${NC}"
    echo ""

    if [ "$PROJECT_TYPE" != "custom" ]; then
        echo "2. Navigate to your project:"
        echo -e "   ${YELLOW}cd $PROJECT_NAME${NC}"
        echo ""

        case $PROJECT_TYPE in
            "svelte-tailwind"|"react-ts"|"vue-vite")
                echo "3. Install dependencies:"
                echo -e "   ${YELLOW}npm install${NC}"
                echo ""
                echo "4. Start development server:"
                echo -e "   ${YELLOW}npm run dev${NC}"
                ;;
            "python")
                echo "3. Activate virtual environment:"
                echo -e "   ${YELLOW}source venv/bin/activate${NC}"
                echo ""
                echo "4. Install dependencies:"
                echo -e "   ${YELLOW}pip install -r requirements.txt${NC}"
                echo ""
                echo "5. Start server:"
                echo -e "   ${YELLOW}uvicorn main:app --reload${NC}"
                ;;
            "rust")
                echo "3. Build and run:"
                echo -e "   ${YELLOW}cargo run${NC}"
                ;;
            "go")
                echo "3. Run server:"
                echo -e "   ${YELLOW}go run main.go${NC}"
                ;;
        esac
    fi

    echo ""
    echo -e "${CYAN}📚 Documentation:${NC} https://github.com/your-repo/universal-lsp"
    echo -e "${CYAN}🐛 Issues:${NC}        https://github.com/your-repo/universal-lsp/issues"
    echo ""
}

#==============================================================================
# Main Execution
#==============================================================================

main() {
    clear
    print_header

    # Interactive prompts
    select_ide
    select_ai_provider
    select_project_type
    get_project_name
    configure_workspace

    # Setup configurations
    case $IDE in
        "zed")
            setup_zed
            ;;
        "vscode")
            setup_vscode
            ;;
        "neovim")
            print_info "Neovim configuration skipped (manual setup required)"
            ;;
        "other")
            print_info "Manual IDE configuration required"
            ;;
    esac

    # Create workspace config if enabled
    if [ "$USE_WORKSPACE_CONFIG" = "yes" ]; then
        create_workspace_config
    fi

    # Scaffold project
    if [ "$PROJECT_TYPE" != "custom" ]; then
        scaffold_project
    fi

    # Print summary
    print_summary
}

# Run main function
main
