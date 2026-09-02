set -e

if [ -z "$__DEVBOX_SKIP_INIT_HOOK_35c2d240cbc4ef69d8f881ee029f36786e87e41cc6ea0ec408fd97de8158c4c7" ]; then
    . "/Users/micro/.local/share/treehouse/.treehouse/apmw-cb9865/1/apmw/.devbox/gen/scripts/.hooks.sh"
fi

just format_impl
