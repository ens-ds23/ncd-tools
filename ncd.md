# Installing NCD

There are two options for installing NCD. One is easy but might not
work, the other is more difficult but more widely applicable.

## Installing from binary (Easy)

Clone ncd-tools

`$ git clone https://github.com/ens-ds23/ncd-tools.git`

Add the following to your `PATH`:

 * For codon `$HOME/ncd-tools/pre-builds/codon`

## Installing from source (Harder)

Installing from source involves a few steps. 

First you will need a Rust environment. Do the following. It will
Create a directory called `~/.cargo` and fill it with stuff.

`$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

Select option 1.

Now get the new code installed on your path.

`$ source ~/.cargo/env`

You can now put this into your bashrc if you will want to do this
over-and-over.

Check it works as follows (printing usage info).

`$ cargo --help`

Clone the ncd-tools repo.

`$ git clone https://github.com/ens-ds23/ncd-tools.git`

Go into the repo directory and build it.

`$ cargo build --release`

When it's built, put the new `ncd-build` binary into the path. (The
below assumes that you've built ncd-tools in `~/ncd-tools`).

`$ export PATH="$HOME/ncd-tools/target/release:$PATH"`

Check it's worked:

`$ ncd-build --help`

You're good to go.

