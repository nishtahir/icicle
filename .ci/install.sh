#!/bin/bash

# Installation script for icicle.
# Adapted from https://github.com/Schniz/fnm/blob/master/.ci/install.sh

set -e

INSTALL_DIR="$HOME/.icicle"
RELEASE="latest"
OS="$(uname -s)"

set_filename() {
  if [ "$OS" = "Darwin" ]; then
    if [ "$(uname -m)" = "arm64" ]; then
      FILENAME="icicle-macos-arm64"
    else
      FILENAME="icicle-macos"
    fi
  elif [ "$OS" = "Linux" ]; then
    # TODO - arm builds?
    FILENAME="icicle-linux"
  fi
}

download_icicle() {
  if [ "$RELEASE" = "latest" ]; then
    URL="https://github.com/nishtahir/icicle/releases/latest/download/$FILENAME.zip"
  else
    URL="https://github.com/nishtahir/icicle/releases/download/$RELEASE/$FILENAME.zip"
  fi

  DOWNLOAD_DIR=$(mktemp -d)
  echo "Downloading $URL..."
  mkdir -p "$INSTALL_DIR" &>/dev/null

  if ! curl --progress-bar --fail -L "$URL" -o "$DOWNLOAD_DIR/$FILENAME.zip"; then
    echo "Download failed.  Check that the release/filename are correct."
    exit 1
  fi

  unzip -q "$DOWNLOAD_DIR/$FILENAME.zip" -d "$DOWNLOAD_DIR"

  if [ -f "$DOWNLOAD_DIR/icicle" ]; then
    mv "$DOWNLOAD_DIR/icicle" "$INSTALL_DIR/icicle"
  else
    mv "$DOWNLOAD_DIR/$FILENAME/icicle" "$INSTALL_DIR/icicle"
  fi

  chmod u+x "$INSTALL_DIR/icicle"
}

ensure_containing_dir_exists() {
  local CONTAINING_DIR
  CONTAINING_DIR="$(dirname "$1")"
  if [ ! -d "$CONTAINING_DIR" ]; then
    echo " >> Creating directory $CONTAINING_DIR"
    mkdir -p "$CONTAINING_DIR"
  fi
}

setup_shell() {
  CURRENT_SHELL="$(basename "$SHELL")"

  if [ "$CURRENT_SHELL" = "zsh" ]; then
    CONF_FILE=${ZDOTDIR:-$HOME}/.zshrc
    ensure_containing_dir_exists "$CONF_FILE"
    echo "Installing for Zsh. Appending the following to $CONF_FILE:"
    echo ""
    echo '  # Icicle https://github.com/nishtahir/icicle/'
    echo '  export PATH='"$INSTALL_DIR"':$PATH'
    echo '  eval "`icicle env`"'

    echo '' >>$CONF_FILE
    echo '# Icicle https://github.com/nishtahir/icicle/' >>$CONF_FILE
    echo 'export PATH='$INSTALL_DIR':$PATH' >>$CONF_FILE
    echo 'eval "`icicle env`"' >>$CONF_FILE
  elif [ "$CURRENT_SHELL" = "fish" ]; then
    CONF_FILE=$HOME/.config/icicle/conf.d/icicle.fish
    ensure_containing_dir_exists "$CONF_FILE"
    echo "Installing for Fish. Appending the following to $CONF_FILE:"
    echo ""
    echo '  # Icicle https://github.com/nishtahir/icicle/'
    echo '  set PATH '"$INSTALL_DIR"' $PATH'
    echo '  icicle env | source'

    echo '# Icicle https://github.com/nishtahir/icicle/' >>$CONF_FILE
    echo 'set PATH '"$INSTALL_DIR"' $PATH' >>$CONF_FILE
    echo 'icicle env | source' >>$CONF_FILE
  elif [ "$CURRENT_SHELL" = "bash" ]; then
    if [ "$OS" = "Darwin" ]; then
      CONF_FILE=$HOME/.profile
    else
      CONF_FILE=$HOME/.bashrc
    fi
    ensure_containing_dir_exists "$CONF_FILE"
    echo "Installing for Bash. Appending the following to $CONF_FILE:"
    echo ""
    echo '  # Icicle https://github.com/nishtahir/icicle/'
    echo '  export PATH='"$INSTALL_DIR"':$PATH'
    echo '  eval "`icicle env`"'

    echo '' >>$CONF_FILE
    echo '# Icicle https://github.com/nishtahir/icicle/' >>$CONF_FILE
    echo 'export PATH='"$INSTALL_DIR"':$PATH' >>$CONF_FILE
    echo 'eval "`icicle env`"' >>$CONF_FILE
  fi
}

set_filename
download_icicle
setup_shell

set -e
