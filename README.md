#ViHex

This is a hex editor written in Rust that aims to have a similar style of
navigation to Vim.

The project is in early stages, but so far, vihex can open, edit, and save
files. The intention is for the editor to modify existing addresses. I
may eventually add the ability to add/remove bytes, but not until other goals
are met.

This editor uses the [Cursive](https://github.com/gyscos/Cursive) crate to help 
build its text-user-interface (TUI). The version of the library we're using 
here has been modified somewhat. The main change is that we've added a custom 
type called HexArea, which handles most of the editor functions. It's based on 
the TextArea type that's built into the original Cursive library, but it's 
been modified significantly to suit the purposes of this project.
