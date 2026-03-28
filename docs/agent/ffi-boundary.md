
# FFI Boundary

## Allowed API

- analyze_file(path, config_json)
- backend_version()

## Design rules

- return JSON string only
- no complex struct mapping
- minimal unsafe usage
- single-call-per-file preferred

## Forbidden

- exposing full Essentia API
- multiple fine-grained FFI calls
