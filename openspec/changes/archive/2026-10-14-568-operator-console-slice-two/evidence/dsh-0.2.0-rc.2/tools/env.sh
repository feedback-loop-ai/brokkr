# shellcheck shell=bash
# Sourced by the probe's drivers, not run: the isolation environment.
S=.
export DSH_HOME=$S/home XDG_DATA_HOME=$S/xdg/data XDG_CACHE_HOME=$S/xdg/cache XDG_CONFIG_HOME=$S/xdg/config XDG_STATE_HOME=$S/xdg/state
export BROKKR_PROBE_LOG=$S/probe-host.log
export BROKKR_STUB_KEY=stub-not-a-credential
export npm_config_cache=$S/xdg/cache/npm
