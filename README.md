# StatusWatch

監控 IoT 平台上各專案（project）裝置狀態變化，並在狀態轉換時透過 Discord Webhook 發送通知的輕量服務。單一靜態執行檔、無需 Node.js/資料庫伺服器，內建 SQLite 與前端資源。

## 技術架構

- **後端**：Rust + [axum](https://github.com/tokio-rs/axum)（web framework）+ [maud](https://maud.lambda.xyz/)（SSR HTML 模板）
- **前端互動**：[htmx](https://htmx.org/)（CDN 腳本已 vendor 進 repo，不在執行期連外）
- **樣式**：Tailwind CSS v4（編譯後的 CSS 直接內嵌進執行檔，非 CDN）
- **資料庫**：SQLite（透過 [sea-orm](https://www.sea-ql.org/SeaORM/)），schema 於啟動時自動建立（`CREATE TABLE IF NOT EXISTS`），無需另外跑 migration
- **輪詢**：`src/poller.rs` 每 60 秒並行輪詢所有 `poll_enabled` 的 project，比對裝置狀態並在狀態轉換時觸發通知

資料模型關係：`Endpoint`（IoT 平台端點）→ `Project`（平台上的專案）→ 裝置狀態輪詢 + `NotifyTarget`（本地設定的 Discord Webhook，當 project 的通知來源設為 local 時使用）。若 project 通知來源設為 remote，則改讀取該 IoT 平台自身的 `ActiveNotify` 設定（操作方式是把某個裝置的 LINE 通知 slot 的 `to` 欄位填入 Discord Webhook URL）。

## 快速開始（Docker，建議方式）

```bash
docker compose up -d
```

- 服務會監聽在 `http://localhost:3000`
- SQLite 資料庫存放在具名 volume `statuswatch-data`（掛載於容器內 `/data`），不會隨容器重建而遺失
- 若部署環境需要透過 proxy 才能連到 IoT 平台，取消 [compose.yml](compose.yml) 中 `HTTPS_PROXY`/`NO_PROXY` 那段的註解並依需求調整

自行建置映像（而非使用 `ghcr.io/e8035669/statuswatch` 上的既有映像）：

```bash
docker compose build
docker compose up -d
```

### 環境變數

| 變數 | 預設值 | 說明 |
| --- | --- | --- |
| `DATABASE_URL` | `sqlite://statuswatch.db?mode=rwc`（Docker 內為 `sqlite:///data/statuswatch.db?mode=rwc`） | SQLite 連線字串 |
| `RUST_LOG` | `info` | 日誌等級，支援 [`tracing-subscriber` EnvFilter](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/struct.EnvFilter.html) 語法，例如 `RUST_LOG=statuswatch=debug` |

服務埠目前固定為 `3000`（寫死在 [src/main.rs](src/main.rs)，非環境變數控制）。

## 本機開發（不用 Docker）

需要 Rust 工具鏈（版本見 [Cargo.toml](Cargo.toml) 的 `edition`，建議用最新 stable）。

```bash
cargo run
```

- 預設會在專案根目錄產生 `statuswatch.db`（已列在 `.gitignore`，不會進版控）
- 若要改連線位置：`DATABASE_URL=sqlite://dev.db?mode=rwc cargo run`

其他常用指令：

```bash
cargo check          # 快速型別檢查
cargo build --release
cargo test
```

## 使用方式（網頁操作流程）

啟動後開啟 `http://localhost:3000`，設定順序如下：

1. **Endpoints**（`/settings/endpoints`）：新增一個 IoT 平台端點，填入名稱、Base URL、以及該平台 API 類型（General / Edge）
2. **Projects**（`/settings/endpoints/{id}/projects`）：在某個 endpoint 底下新增要監控的 project（填入平台上的 `project_key`），並選擇：
   - **是否啟用輪詢**（`poll_enabled`）
   - **通知來源**（`notify_source`）：
     - `remote`：沿用該 IoT 平台自身設定的 ActiveNotify（LINE slot 的 `to` 欄位需為 Discord Webhook URL）
     - `local`：改用本服務自己的 Notify Targets 設定
3. **Notify Targets**（`/settings/projects/{project_id}/notify-targets`，僅當 project 設為 `local` 時需要）：新增 Discord Webhook URL，並可自訂訊息樣板，支援以下佔位符：
   - `{device_id}`、`{device_name}`、`{status}`、`{old_status}`、`{time}`
4. **Dashboard**（`/`）：查看所有 project 目前各裝置的最新狀態
5. **History**（`/history`）：查看歷史通知發送紀錄（成功/失敗）

輪詢邏輯：裝置第一次被看到時只會「靜默記錄」目前狀態，不會發通知；之後狀態若有變化才會觸發通知並寫入 `notify_history`。

## 專案回頭維護指南

> 目標讀者：幾個月後回來維護、已經忘記細節的自己（或是下一位AI模型:D）。

### 專案結構速覽

```
src/
  main.rs        # 啟動流程、graceful shutdown、DB 連線 + poll loop 併行執行
  db.rs          # DB 連線 + schema 自動建立（無 migration 檔案）
  state.rs       # AppState（DB connection + reqwest Client）
  poller.rs      # 60 秒輪詢迴圈，比對裝置狀態
  notify.rs      # 組裝訊息 + 發送 Discord Webhook + 寫入 notify_history
  assets.rs      # 用 include_str! 內嵌 static/css/app.css、static/js/htmx.min.js
  entities/      # sea-orm 實體：endpoint / project / device_status / notify_target / notify_history
  views/         # 每個頁面/路由一個檔案，回傳 maud Markup（含 htmx 局部更新片段）
  components/    # 共用 UI 片段（badge、nav）
assets/tailwind.css        # Tailwind 原始輸入檔（@import "tailwindcss";）
static/css/app.css         # Tailwind 編譯輸出，「已提交進版控」，不要手改
static/js/htmx.min.js      # vendor 進來的 htmx，「已提交進版控」，不要手改
scripts/update-frontend-assets.sh   # 見下方「更新前端資源」
```

### 更新前端資源（Tailwind CSS / htmx）

本機**沒有安裝 Node.js/npm**，`static/css/app.css` 與 `static/js/htmx.min.js` 都是離線建置後直接提交進版控、並在編譯期用 `include_str!` 內嵌進執行檔的（見 [src/assets.rs](src/assets.rs)），執行期完全不連外、不依賴檔案系統。

當你改了 `src/**/*.rs` 裡的 Tailwind class，或想要升級 htmx / Tailwind 版本時，執行：

```bash
scripts/update-frontend-assets.sh [htmx_version] [tailwind_version]
```

例如：

```bash
scripts/update-frontend-assets.sh           # 用預設版本重新整理（沒改版本號時，用來重新掃描 class 並重建 app.css）
scripts/update-frontend-assets.sh 2.0.4 v4.3.3   # 明確指定版本
```

腳本做的事：

1. 下載指定版本的 `htmx.min.js` 覆蓋 `static/js/htmx.min.js`
2. 下載對應 OS/arch 的 Tailwind v4 standalone CLI 到 `.tools/tailwindcss`（已 gitignore，不進版控）
3. 用該 CLI 以 `assets/tailwind.css` 為輸入，自動掃描專案內所有檔案找出用到的 class，重新產出 `static/css/app.css`（`--minify`）

跑完後**務必**：

```bash
git diff static/
```

確認差異合理後再 commit。這個腳本刻意**不是** `build.rs`，因為那樣每次 `cargo build` 都要連網（或得 vendor 一份上百 MB 的平台專屬二進位檔），會破壞「離線／單一執行檔」這個部署目標；只有要升版或改了 Tailwind class 時才需要手動跑一次。

Tailwind v4 不需要 `tailwind.config.js` 或 content glob 設定，CLI 會自動掃描專案檔案。

### Docker 映像建置細節

- 多階段建置：`rust-musl-cross`（依 `TARGETARCH` 選對應 arch 的 stage，跨平台建置時勿改動 `FROM --platform=$BUILDPLATFORM ...` 那段邏輯，註解裡有說明為何不能用 `${TARGETARCH}` 直接代入 `--platform`）→ `FROM scratch` 最終階段
- 最終映像只有一個靜態二進位檔，沒有 CA 憑證、沒有 `static/` 目錄（因為都內嵌了），約 24MB
- CI（`.github/workflows/docker-publish.yml`）在 push 到 `main`/打 tag/開 PR 時建置；PR 只建置不 push；push 到 `main`/tag 時會推到 `ghcr.io/<owner>/<repo>`，使用內建的 `GITHUB_TOKEN`（需要 repo 設定 Actions 的 `packages: write` 權限）

### 常見坑（維護時容易忘記的地方）

- `static/css/app.css`、`static/js/htmx.min.js` 是**產出物**但**有提交進版控**（因為要被 `include_str!` 內嵌），不要以為是 build artifact 就加進 `.gitignore` 或手動編輯它們——要改就跑 `scripts/update-frontend-assets.sh`
- `statuswatch.db`（本機開發用的 SQLite 檔）已 gitignore，不會被提交
- 新增 project 時若忘記把 `notify_source` 設對（remote/local），通知會送不出去或送去錯的地方
- reqwest 的 proxy 支援是自動讀 `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` 環境變數（`state::build_client()` 沒有呼叫 `.no_proxy()`），不需要額外寫程式碼處理
- 若之後要再內嵌新的靜態資源，比照 `src/assets.rs` 現有寫法用 `include_str!`/`include_bytes!`，並把來源檔案放進 `static/`（保留一份在 repo 裡，不要在請求期才去外部抓）
