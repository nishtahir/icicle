#!/bin/bash
# shellcheck disable=SC2129
# shellcheck disable=SC2016

set -e

help() {
  echo "Usage: installer.sh --release <release> --install-dir <install-dir> --shell <shell>"
  echo ""
  echo "Options:"
  echo "  --release <release>    The release version to install. Defaults to latest."
  echo "  --install-dir <dir>    The directory to install icicle to. Defaults to ~/.icicle."
  echo "  --skip-shell           Skip adding icicle to your shell profile."
  echo "  --help                 Print this help message."
}

INSTALL_DIR="$HOME/.icicle"
RELEASE="latest"
SKIP_SHELL=0

OS="$(uname -s)"
ARCH="$(uname -m)"

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --release)
      RELEASE="$2"
      shift
      shift
      ;;
    --install-dir)
      INSTALL_DIR="$2"
      shift
      shift
      ;;
    --skip-shell)
      SKIP_SHELL=1
      shift
      ;;
    --help)
      help
      exit 0
      ;;
    *)
      echo "Unknown argument: $1"
      help
      exit 1
      ;;
  esac
done

# determine the filename based on the user's OS and architecture
# macos is Darwin, linux is Linux
# x86_64 is amd64, arm64 is arm64, aaarch64 is arm64

if [[ "$OS" == "Darwin" ]]; then
  OS="darwin"
elif [[ "$OS" == "Linux" ]]; then
  OS="linux"
else
  echo "Unsupported OS: $OS"
  exit 1
fi

if [[ "$ARCH" == "x86_64" ]]; then
  ARCH="amd64"
elif [[ "$ARCH" == "arm64" || "$ARCH" == "aarch64" ]]; then
  ARCH="arm64"
else
  echo "Unsupported architecture: $ARCH"
  exit 1
fi

FILENAME="icicle-$OS-$ARCH"
if [ -n "$ICICLE_DOWNLOAD_URL" ]; then
    URL="$ICICLE_DOWNLOAD_URL"
elif [ "$RELEASE" = "latest" ]; then
    URL="https://github.com/nishtahir/icicle/releases/latest/download/$FILENAME"
else
    URL="https://github.com/nishtahir/icicle/releases/download/$RELEASE/$FILENAME"
fi

echo "Downloading $URL..."

# Create the install directory if it doesn't exist
DOWNLOAD_DIR=$(mktemp -d)
mkdir -p "$INSTALL_DIR" &>/dev/null

# Download the binary
if ! curl --progress-bar --fail -L "$URL" -o "$DOWNLOAD_DIR/$FILENAME"; then
  echo "Download failed.  Check that the release/filename are correct."
  exit 1
fi

mv "$DOWNLOAD_DIR/$FILENAME" "$INSTALL_DIR/icicle"
chmod u+x "$INSTALL_DIR/icicle"

# if the user has requested to skip shell setup, exit now
if [[ "$SKIP_SHELL" == "1" ]]; then
  echo "Skipping shell profile setup."
  exit 0
fi

# Add icicle to the user's shell profile
CURRENT_SHELL="$(basename "$SHELL")"
MARKER="# Icicle https://github.com/nishtahir/icicle/"

ensure_containing_dir_exists() {
  local CONTAINING_DIR
  CONTAINING_DIR="$(dirname "$1")"
  if [ ! -d "$CONTAINING_DIR" ]; then
    echo " >> Creating directory $CONTAINING_DIR"
    mkdir -p "$CONTAINING_DIR"
  fi
}

# Bash reads ~/.bashrc for interactive non-login shells (most Linux terminals) but only the
# first of ~/.bash_profile, ~/.bash_login and ~/.profile for login shells (every macOS
# terminal). Use whichever file the shell is going to read.
bash_conf_file() {
  if [ "$OS" = "darwin" ]; then
    for candidate in "$HOME/.bash_profile" "$HOME/.bash_login" "$HOME/.profile"; do
      if [ -f "$candidate" ]; then
        echo "$candidate"
        return
      fi
    done
    echo "$HOME/.bash_profile"
  else
    echo "$HOME/.bashrc"
  fi
}

# append_block <conf file> <shell name> <line>...
# Does nothing if the file already has an icicle block, so running the installer again
# (e.g. to upgrade) doesn't stack up duplicates.
append_block() {
  local CONF_FILE="$1" SHELL_NAME="$2"
  shift 2

  ensure_containing_dir_exists "$CONF_FILE"
  if [ -f "$CONF_FILE" ] && grep -qF "$MARKER" "$CONF_FILE"; then
    echo "Icicle is already set up for $SHELL_NAME in $CONF_FILE. Skipping."
    return
  fi

  echo "Installing for $SHELL_NAME."
  echo "Appending the following to $CONF_FILE:..."
  echo ''
  {
    echo ''
    echo "$MARKER"
    for line in "$@"; do
      echo "$line"
    done
    echo ''
  } | tee -a "$CONF_FILE" | sed 's/^/  /'
}

if [ "$CURRENT_SHELL" = "zsh" ]; then
  append_block "${ZDOTDIR:-$HOME}/.zshrc" "Zsh" \
    'export PATH='"$INSTALL_DIR"':$PATH' \
    'eval "$(icicle env)"'

elif [ "$CURRENT_SHELL" = "fish" ]; then
  append_block "$HOME/.config/fish/conf.d/icicle.fish" "Fish" \
    'set PATH '"$INSTALL_DIR"' $PATH' \
    'icicle env | source'

elif [ "$CURRENT_SHELL" = "bash" ]; then
  append_block "$(bash_conf_file)" "Bash" \
    'export PATH='"$INSTALL_DIR"':$PATH' \
    'eval "$(icicle env)"'

else
  echo "Unsupported shell '$CURRENT_SHELL'. Add the following to your shell profile:"
  echo ''
  echo '  export PATH='"$INSTALL_DIR"':$PATH'
  echo '  eval "$(icicle env)"'
fi
