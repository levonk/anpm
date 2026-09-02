set -e

if [ -z "$__DEVBOX_SKIP_INIT_HOOK_08281178dec00b5d25a4ddf52299463d0b7dc2e25284999cc70276c2bb256d71" ]; then
    . "/Users/micro/p/gh/levonk/apmw/.devbox/gen/scripts/.hooks.sh"
fi

just dev_impl
