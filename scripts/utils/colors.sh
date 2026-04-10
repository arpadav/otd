#!/bin/bash
# Author: aav
# --------------------------------------------------
# Description:
#   Gets the ANSI colors, if colors are supported within
# the terminal. This is so it can be read in like:
#
#   `read -r CYAN_BOLD GREEN YELLOW RESET <<< "$(bash scripts/colors.sh CYAN green yellow reset)"`
#
# --------------------------------------------------
# Usage:
#   `bash scripts/colors.sh [<color-name> ...]`
# --------------------------------------------------
# * `<color-name>`: space delimited colors, if capitalized
#   then the color is bold.
# --------------------------------------------------

# --------------------------------------------------
# check for color support
# --------------------------------------------------
can_color=false
if [ -n "$TERM" ] && [ "$TERM" != "dumb" ]; then
	if ncolors=$(tput colors 2>/dev/null); then
		ncolors=$(echo "$ncolors" | tr -d '[:space:]')
		if [[ "$ncolors" =~ ^[0-9]+$ ]]; then
			if [ "$ncolors" -ge 8 ]; then
				can_color=true
			fi
		fi
	fi
fi
if [ "$can_color" = false ] && [ -n "$COLORTERM" ]; then
	case "$COLORTERM" in
	truecolor | 24bit | rgb)
		can_color=true
		;;
	esac
fi
if [ "$can_color" = false ]; then
	case "$TERM" in
	xterm-color | xterm-256color | screen | tmux | linux | cygwin | ansi | rxvt | rxvt-unicode | alacritty | kitty)
		can_color=true
		;;
	esac
fi

# --------------------------------------------------
# return
# --------------------------------------------------
for color in "$@"; do
	if [ "$can_color" = true ]; then
		# --------------------------------------------------
		# get the ANSI escape code for a given color name
		#
		# if the color name is not recognized, the function returns the escape code
		# for resetting the text color
		# --------------------------------------------------
		case "$color" in
		black) code='\\033[0;30m' ;;
		red) code='\\033[0;31m' ;;
		green) code='\\033[0;32m' ;;
		yellow) code='\\033[0;33m' ;;
		gold) code='\\033[0;38;5;136m' ;;
		blue) code='\\033[0;34m' ;;
		magenta) code='\\033[0;35m' ;;
		cyan) code='\\033[0;36m' ;;
		white) code='\\033[0;37m' ;;
		BLACK) code='\\033[1;30m' ;;
		RED) code='\\033[1;31m' ;;
		GREEN) code='\\033[1;32m' ;;
		YELLOW) code='\\033[1;33m' ;;
		GOLD) code='\\033[1;38;5;136m' ;;
		BLUE) code='\\033[1;34m' ;;
		MAGENTA) code='\\033[1;35m' ;;
		CYAN) code='\\033[1;36m' ;;
		WHITE) code='\\033[1;37m' ;;
		*) code='\\033[0m' ;;
		esac
	else
		code=''
	fi
	printf "$code "
done
