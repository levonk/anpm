#!/usr/bin/bash
set -e

## Install nvm
vet https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh

## Install pnpm
vet  https://get.pnpm.io/install.sh

## Install bun
vet https://bun.sh/install

## Install Yarn Berry
curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | sudo apt-key add -
echo "deb https://dl.yarnpkg.com/debian stable main" | sudo tee /etc/apt/sources.list.d/yarn.list
sudo apt update
sudo apt install yarn
