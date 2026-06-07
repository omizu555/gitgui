use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

/// リポジトリパス単位の排他ロック。
///
/// コマンドの非同期化により、自動フェッチとユーザー操作（checkout / pull など）が
/// 同一リポジトリで真に並行実行されると index.lock 競合が発生する。
/// 書き込み系操作はこのロックでリポジトリ単位に直列化する。
///
/// ロック本体は tokio::sync::Mutex を使う。`lock_owned()` が返す
/// OwnedMutexGuard は Send なので、spawn_blocking の完了まで await をまたいで保持できる
/// （std::sync::Mutex の guard は !Send のため不可）。
#[derive(Default)]
pub struct RepoLocks {
    map: Mutex<HashMap<String, Arc<AsyncMutex<()>>>>,
}

impl RepoLocks {
    /// 指定リポジトリのロックを取得する（なければ作成）。
    /// 内側の std::Mutex は HashMap 操作の間だけ保持し、await をまたがない。
    pub fn lock_for(&self, path: &str) -> Arc<AsyncMutex<()>> {
        let key = normalize_repo_key(path);
        let mut map = self.map.lock().unwrap();
        map.entry(key)
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone()
    }
}

/// パスをロックキーに正規化する（区切り文字と Windows の大文字小文字差を吸収）。
fn normalize_repo_key(path: &str) -> String {
    let t = path.replace('\\', "/");
    let t = t.trim_end_matches('/');
    if cfg!(windows) {
        t.to_lowercase()
    } else {
        t.to_string()
    }
}

/// キャンセル可能な実行中 git プロセスの管理。
/// operation_id → PID を登録し、git_cancel コマンドからプロセスツリーごと kill する。
#[derive(Default)]
pub struct RunningOps {
    pids: Mutex<HashMap<String, u32>>,
    cancelled: Mutex<HashSet<String>>,
}

impl RunningOps {
    pub fn register(&self, op_id: &str, pid: u32) {
        self.pids.lock().unwrap().insert(op_id.to_string(), pid);
    }

    pub fn unregister(&self, op_id: &str) {
        self.pids.lock().unwrap().remove(op_id);
    }

    pub fn pid_of(&self, op_id: &str) -> Option<u32> {
        self.pids.lock().unwrap().get(op_id).copied()
    }

    /// キャンセル要求としてマークする（kill 後にコマンド側が判別するため）
    pub fn mark_cancelled(&self, op_id: &str) {
        self.cancelled.lock().unwrap().insert(op_id.to_string());
    }

    /// キャンセル済みかを確認しつつフラグを消費する
    pub fn take_cancelled(&self, op_id: &str) -> bool {
        self.cancelled.lock().unwrap().remove(op_id)
    }
}
