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
if [ "$RELEASE" = "latest" ]; then
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

ensure_containing_dir_exists() {
  local CONTAINING_DIR
  CONTAINING_DIR="$(dirname "$1")"
  if [ ! -d "$CONTAINING_DIR" ]; then
    echo " >> Creating directory $CONTAINING_DIR"
    mkdir -p "$CONTAINING_DIR"
  fi
}

if [ "$CURRENT_SHELL" = "zsh" ]; then
  CONF_FILE=${ZDOTDIR:-$HOME}/.zshrc
  ensure_containing_dir_exists "$CONF_FILE"
  echo 'Installing for Zsh.'
  echo "Appending the following to $CONF_FILE:..."
  echo ''
  echo '  # Icicle https://github.com/nishtahir/icicle/'
  echo '  export PATH='"$INSTALL_DIR"':$PATH'
  echo '  eval "$(icicle env)"'
  echo ''
  
  echo '' >> "$CONF_FILE"
  echo '# Icicle https://github.com/nishtahir/icicle/' >> "$CONF_FILE"
  echo 'export PATH='"$INSTALL_DIR"':$PATH' >>"$CONF_FILE"
  echo 'eval "$(icicle env)"' >>"$CONF_FILE"
  echo '' >> "$CONF_FILE"

elif [ "$CURRENT_SHELL" = "fish" ]; then
  CONF_FILE=$HOME/.config/icicle/conf.d/icicle.fish
  ensure_containing_dir_exists "$CONF_FILE"
  echo 'Installing for Fish.'
  echo "Appending the following to $CONF_FILE:..."
  echo ''
  echo '  # Icicle https://github.com/nishtahir/icicle/'
  echo '  set PATH '"$INSTALL_DIR"' $PATH'
  echo '  icicle env | source'
  echo ''

  echo '' >> "$CONF_FILE"
  echo '# Icicle https://github.com/nishtahir/icicle/' >>"$CONF_FILE"
  echo 'set PATH '"$INSTALL_DIR"' $PATH' >>"$CONF_FILE"
  echo 'icicle env | source' >>"$CONF_FILE"
  echo '' >> "$CONF_FILE"

elif [ "$CURRENT_SHELL" = "bash" ]; then
  if [ "$OS" = "Darwin" ]; then
    CONF_FILE=$HOME/.profile
  else
    CONF_FILE=$HOME/.bashrc
  fi
  ensure_containing_dir_exists "$CONF_FILE"
  echo 'Installing for Bash.'
  echo "Appending the following to $CONF_FILE:..."
  echo ''
  echo '  # Icicle https://github.com/nishtahir/icicle/'
  echo '  export PATH='"$INSTALL_DIR"':$PATH'
  echo '  eval "$(icicle env)"'
  echo ''

  echo '' >> "$CONF_FILE"
  echo '# Icicle https://github.com/nishtahir/icicle/' >> "$CONF_FILE"
  echo 'export PATH='"$INSTALL_DIR"':$PATH' >> "$CONF_FILE"
  echo 'eval "$(icicle env)"' >> "$CONF_FILE"
  echo '' >> "$CONF_FILE"

fi

set +e