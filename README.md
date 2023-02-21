# WIP!
There are several "unwraps" that need to be handled correctly and ergonomic upgrades to be made.

# Clipsync

Clipsync is a neovim plugin that pushes the content of the `+` buffer to a remote host's clipboard any time content is yanked into it.

It allows you to yank into the `+` buffer while in a remote `nvim` session and have that yanked content show up in your host system's clipboard.

## Requirements
- A (relatively new) Rust toolchain installed on both the remote and host machines.
- Cmake

## Installation

### Remote Machine
On the _remote_ machine you wish to receive clipboard updates from, install the plugin:

#### Plug
`Plug "masonj188/clipsync", {'do': 'cargo install --path .' }`

#### Packer
`use { "masonj188/clipsync", run = 'cargo install --path .' }`

For the post-install scripts to work, `cargo` must be in your path. If `~/.cargo/bin` is _not_ in your path, set `g:clipsync_bin` to the full path of the `clipsync-plugin` binary. I.e. `let g:clipsync_bin = '/home/foo/.cargo/bin/clipsync-plugin'` or `vim.g.clipsync_bin='/home/foo/.cargo/bin/clipsync-plugin'` for lua configs.

### Host Machine
On the machine you want the clipboard to be updated on:

`cargo install clipsync`

Then run the server `clipsync-receiver`.

## Connecting to the remote server
With the server running on the host machine, in neovim run `:ClipsyncConnect "http://<hostname/ip>:8089"`, modified to match the hostname or IP of the host machine running the server.

Clipsync itself does _not_ take care of encryption/TLS. If you're running it across the public internet, consider using `wireguard` or another VPN/tunneling solution to make sure the contents of your clipboard are not going across the public internet in plain text. (Wireguard also makes it easier to get an IP address for your host machine)
