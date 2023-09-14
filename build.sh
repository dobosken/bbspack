#!/bin/bash
#sudo systemctl start docker
cargo build -r
cross build --target x86_64-pc-windows-gnu --release
