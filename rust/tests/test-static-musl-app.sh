#!/bin/sh
set -eu

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <interactive-executable> <log-directory>" >&2
    exit 1
fi
exe=$1
logs=$2
for tool in readelf strace timeout xdotool xvfb-run; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Static musl smoke test requires $tool." >&2
        exit 1
    fi
done
if readelf -l "$exe" | grep -q ' INTERP ' || readelf -d "$exe" | grep -q '(NEEDED)'; then
    echo "The application has dynamic ELF dependencies." >&2
    exit 1
fi
mkdir -p "$logs"
exe=$(cd "$(dirname "$exe")" && pwd -P)/$(basename "$exe")
logs=$(cd "$logs" && pwd -P)
export RUSTOLONIA_MUSL_SMOKE_EXE="$exe" RUSTOLONIA_MUSL_SMOKE_LOGS="$logs"

xvfb-run -a -e "$logs/xvfb.log" sh -eu -c '
    exe=$RUSTOLONIA_MUSL_SMOKE_EXE
    logs=$RUSTOLONIA_MUSL_SMOKE_LOGS
    unset DBUS_SESSION_BUS_ADDRESS
    timeout -k 2 90 strace -f -o "$logs/trace.log" -e trace=open,openat,openat2 \
        "$exe" >"$logs/stdout.log" 2>"$logs/stderr.log" &
    app=$!
    trap "kill $app 2>/dev/null || true; wait $app 2>/dev/null || true" EXIT
    window=
    for attempt in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
        window=$(xdotool search --onlyvisible --name "Rustolonia static musl probe" 2>/dev/null | head -1 || true)
        [ -n "$window" ] && break
        if ! kill -0 "$app" 2>/dev/null; then
            cat "$logs/stdout.log" "$logs/stderr.log"
            echo "The static app exited before creating its window." >&2
            exit 1
        fi
        sleep 1
    done
    if [ -z "$window" ]; then
        cat "$logs/stdout.log" "$logs/stderr.log"
        echo "The static app did not create its window." >&2
        exit 1
    fi
    xdotool windowfocus --sync "$window"
    xdotool mousemove --window "$window" 80 20 click 1
    xdotool key --window "$window" ctrl+a
    xdotool type --window "$window" --delay 30 static-input
    sleep 1
    xdotool mousemove --window "$window" 40 150 click 1
    picker=
    for attempt in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do
        picker=$(xdotool search --onlyvisible --name "Rustolonia static picker probe" 2>/dev/null | head -1 || true)
        [ -n "$picker" ] && break
        if ! kill -0 "$app" 2>/dev/null; then
            break
        fi
        sleep 1
    done
    if [ -z "$picker" ]; then
        cat "$logs/stdout.log" "$logs/stderr.log"
        echo "The static app did not open its file picker." >&2
        exit 1
    fi
    width=$(xdotool getwindowgeometry --shell "$picker" | sed -n "s/^WIDTH=//p")
    height=$(xdotool getwindowgeometry --shell "$picker" | sed -n "s/^HEIGHT=//p")
    xdotool windowfocus --sync "$picker"
    sleep 2
    xdotool mousemove --window "$picker" "$((width - 40))" "$((height - 26))" click 1
    for attempt in 1 2 3 4 5 6 7 8 9 10; do
        grep -q "^picker=cancelled$" "$logs/stdout.log" && break
        sleep 1
    done
    grep -q "^clipboard=static-clipboard$" "$logs/stdout.log"
    grep -q "^picker=cancelled$" "$logs/stdout.log"
    xdotool windowfocus --sync "$window"
    xdotool mousemove --window "$window" 40 80 click 1
    if ! wait "$app"; then
        cat "$logs/stdout.log" "$logs/stderr.log"
        echo "The static app did not exit successfully after the click." >&2
        exit 1
    fi
    trap - EXIT
    grep -q "^edited=static-input$" "$logs/stdout.log"
    [ "$(grep -c "^clicked$" "$logs/stdout.log")" -eq 1 ]
    [ "$(grep -c "^stopped$" "$logs/stdout.log")" -eq 1 ]
    if grep -E "\.so([.\"][^\"]*)?\".*= [0-9]+" "$logs/trace.log"; then
        echo "The static app opened a shared object." >&2
        exit 1
    fi
'
echo 'Static musl app accepted input, used the clipboard and file picker, and exited without shared-object loads.'
