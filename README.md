# lume-calendar-sidecar

Downloads and parses ICS calendar feeds from Google Calendar or compatible sources.

Produces `CalendarPayload` payloads conforming to the VZGLYD sidecar channel ABI.

This sidecar is designed to be reusable. Any slide can depend on it via git and receive data payloads through the standard channel ABI.

## Poll Interval

Every 15 minutes.

## Payload Format

`CalendarPayload` serialized as JSON bytes.

## Environment Variables

| Variable | Description |
|---|---|
| `GCAL_ICS_URL` | ICS feed URL (required) |
| `GCAL_TZ` | Timezone (default: Australia/Melbourne) |

## Usage

Build the sidecar:

```bash
cargo build --target wasm32-wasip1 --release
```

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
