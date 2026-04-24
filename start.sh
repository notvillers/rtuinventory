#!/bin/bash

set -e

script_dir=$(dirname "$")
cd $script_dir

cp ./target/release/rtuinventory

./rtuinventory