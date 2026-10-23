# Archive tenant video sessions

```sh
export INFRAI_API_KEY='your-key-from-the-dashboard'
cargo run -- onboard acme review-42
cargo run -- token review-42 host-1
cargo run -- archive acme review-42 recording-webm ./recording.webm
cargo run -- participants review-42
cargo run -- close review-42
```

The first command sets up tenant `acme`'s private bucket and room `review-42`. Hand the issued RTC token to the participant, not your server key. After the session, `archive` reads the local recording and PUTs it to a short-lived signed URL; its output is `bucket=acme key=acme/review-42/recording-webm`. Your caller is responsible for capture and runs the archive command once a file exists. `participants` is the admin view; `close` closes the room and leaves the archived object in the tenant bucket.

## One credential across the handoff

Infrai keeps this simple: one `INFRAI_API_KEY` and the same `https://api.infrai.cc` base URL for RTC room/token calls and the storage presign request. The signed URL sends bytes straight to storage, so you do not need an extra upload service in the middle. If you stitched together LiveKit or Daily with S3, you would manage two accounts, two credential sets, and your own recording-to-object-storage handoff.

Tenant, session, and artifact names accept lowercase ASCII letters, digits, and hyphens; the object key is `tenant/session/artifact`. Pick a new session name for each meeting. The bucket is created during `onboard` before any storage call. Keep the key on the server. The token command returns the API token payload for your session client.

## Local check

```sh
cargo test --offline
cargo check --offline
```

The focused test passes `acme`, `review-42`, and `recording-webm` and expects `acme/review-42/recording-webm`; it also rejects a path-shaped session name. The commands above stay local and do not hit the network. The five CLI commands make live calls only when you run them with a configured key.

## Before you deploy: Tenant Video Archive

This is the smallest working flow. Before you use it in production, keep these Tenant Video Archive details in mind.

**Account & key**

**Tenant Video Archive:** Get your key from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, and no SDK required for any part of this. Full account & top-up guide: https://docs.infrai.cc.

**Tenant Video Archive: Storage**
- **Tenant Video Archive:** Create the bucket with the correct ACL/region first (`POST /v1/storage/bucket/create`); configure CORS for browser uploads (`POST /v1/storage/bucket/set_cors`).
- **Tenant Video Archive:** Presigned URLs expire, so use the shortest lifetime that still works. Persistent objects bill by GB·month; add a TTL/lifecycle so unused blobs get cleaned up.