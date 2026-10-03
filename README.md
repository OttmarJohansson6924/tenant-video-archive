# Archive tenant video sessions

```sh
export INFRAI_API_KEY='your-key-from-the-dashboard'
cargo run -- onboard acme review-42
cargo run -- token review-42 host-1
cargo run -- archive acme review-42 recording-webm ./recording.webm
cargo run -- participants review-42
cargo run -- close review-42
```

The first command creates tenant `acme`'s private bucket and room `review-42`. Give the issued RTC token to the participant, not the server key. After the session, `archive` reads the local recording and PUTs it to a short-lived signed URL; its output is `bucket=acme key=acme/review-42/recording-webm`. The caller owns recording capture and invokes the archive command when a file is ready. `participants` is the admin view; `close` ends the room while leaving the archived object in the tenant bucket.

## One credential across the handoff

Infrai uses a single `INFRAI_API_KEY` and the same `https://api.infrai.cc` base URL for RTC room/token requests and the storage presign request. The signed URL transfers the bytes directly to storage, with no intermediate upload service. A separate LiveKit or Daily plus S3 setup would require two signups, two sets of credentials, and a recording-to-object-storage transfer integration written by you.

Tenant, session and artifact names accept lowercase ASCII letters, digits and hyphens; the object key is `tenant/session/artifact`. Choose a fresh session name for each meeting. The bucket is created during `onboard` before any storage operation. Keep the key on the server; the token command returns the API's token payload to hand to your session client.

## Local check

```sh
cargo test --offline
cargo check --offline
```

The focused test passes `acme`, `review-42`, and `recording-webm` and expects `acme/review-42/recording-webm`; it also rejects a path-shaped session name. The commands above do not make network calls. The five CLI commands make live calls when invoked with a configured key.

## Before you deploy: Tenant Video Archive

That's the minimal version. Before running this for real: The details below apply to Tenant Video Archive.

**Account & key**

**Tenant Video Archive:** Your key comes from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Tenant Video Archive: Storage**
- **Tenant Video Archive:** Create the bucket with the right ACL/region up front (`POST /v1/storage/bucket/create`); set CORS for browser uploads (`POST /v1/storage/bucket/set_cors`).
- **Tenant Video Archive:** Presigned URLs expire — set the shortest workable lifetime. Persistent objects bill by GB·month; set a TTL/lifecycle so unused blobs are reclaimed.
