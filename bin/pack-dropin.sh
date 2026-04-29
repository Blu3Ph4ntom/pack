#!/bin/bash
# Pack - Drop-in replacement wrapper for gem and bundle commands
#
# Install as symlink to replace gem/bundle:
#   ln -sf /path/to/pack /usr/local/bin/gem
#   ln -sf /path/to/pack /usr/local/bin/bundle
#
# Or install via make:
#   make install-dropin

PACK_BIN="${PACK_BINARY:-/usr/local/bin/pack}"

# Detect if we're being called as 'gem' or 'bundle'
cmd_name=$(basename "$0")

case "$cmd_name" in
    gem)
        # Forward to pack gem <args>
        exec "$PACK_BIN" gem "$@"
        ;;
    bundle)
        # Check if it's 'bundle exec <cmd>'
        if [ "$1" = "exec" ]; then
            shift
            exec "$PACK_BIN" exec "$@"
        else
            # Forward other bundle commands
            exec "$PACK_BIN" "$@"
        fi
        ;;
    pack)
        # Direct pack command
        exec "$PACK_BIN" "$@"
        ;;
    *)
        # Called as something else, just exec pack
        exec "$PACK_BIN" "$@"
        ;;
esac