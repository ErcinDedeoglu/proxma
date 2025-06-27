# gRPC Testing with grpcurl

## Basic Usage

```bash
# List services (no auth)
grpcurl domain:443 list

# List services (with auth)
grpcurl -H "Authorization: Basic $(echo -n 'username:password' | base64)" domain:443 list

# Call a method
grpcurl -H "Authorization: Basic $(echo -n 'username:password' | base64)" \
  -d '{"request": "data"}' \
  domain:443 service.Method
```

## Authentication

grpcurl does **NOT** support `-u` flag. Use manual base64 encoding:

```bash
# ⚠️ IMPORTANT: Use -n flag to avoid newline issues
echo -n "temporal:mypassword" | base64
# Result: dGVtcG9yYWw6bXlwYXNzd29yZA==

# ❌ WRONG: Without -n flag (adds newline)
echo "temporal:mypassword" | base64
# Result: dGVtcG9yYWw6bXlwYXNzd29yZAo= (different!)

# Use in request
grpcurl -H "Authorization: Basic dGVtcG9yYWw6bXlwYXNzd29yZA==" domain:443 list
```

## Avoiding Password Mismatch Issues

**Proxma automatically trims whitespace from passwords**, but for consistency use `echo -n`:

```bash
# ✅ Recommended way
echo -n "username:password" | base64

# ✅ Also works (Proxma trims newlines)
echo "username:password" | base64
```

**Verify your base64 encoding:**

```bash
# Test what your base64 actually contains
echo "your_base64_string" | base64 -d
# Should show: username:password
```

## Common Flags

- `-H "header: value"` - Add headers
- `-d '{"json": "data"}'` - Request data
- `-plaintext` - Use HTTP (not HTTPS)
- `-insecure` - Skip TLS verification
- `-v` - Verbose output

## Troubleshooting

**Error: `unexpected HTTP status code received from server: 204`**
- Old issue: Authentication failing, returning gRPC error correctly
- Check credentials with: `echo "base64string" | base64 -d`

**Error: `rpc error: code = Unauthenticated desc = Authentication required`**
- Correct gRPC error format
- Check username/password in Docker labels

**Error: `flag provided but not defined: -u`**
- grpcurl doesn't support `-u` flag
- Use `-H "Authorization: Basic ..."` instead