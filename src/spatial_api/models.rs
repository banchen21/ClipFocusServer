use actix::{WeakAddr, prelude::*};
use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_web_actors::ws;
use chrono::Local;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

// 房间管理器
pub struct RoomManager {
    // user_id -> session_id -> WeakAddr
    rooms: HashMap<String, HashMap<String, WeakAddr<MyWs>>>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    // 清理指定用户的死亡连接
    fn cleanup_dead_connections(&mut self, user_id: &str) {
        if let Some(sessions) = self.rooms.get_mut(user_id) {
            // 先收集死亡的 session_id
            let dead_sessions: Vec<String> = sessions
                .iter()
                .filter(|(_, weak_addr)| weak_addr.upgrade().is_none())
                .map(|(session_id, _)| session_id.clone())
                .collect();
            
            // 移除死亡的连接
            for session_id in dead_sessions {
                sessions.remove(&session_id);
                println!("🧹 Cleaned up dead session: {}", &session_id[..8]);
            }
            
            // 如果房间为空，移除整个房间
            if sessions.is_empty() {
                self.rooms.remove(user_id);
                println!("🗑️ Room {} is now empty and removed", user_id);
            }
        }
    }

    // 加入房间
    pub fn join_room(&mut self, user_id: &str, session_id: String, addr: Addr<MyWs>) {
        // 先清理死亡连接
        self.cleanup_dead_connections(user_id);
        
        let sessions = self
            .rooms
            .entry(user_id.to_string())
            .or_insert_with(HashMap::new);
        
        sessions.insert(session_id.clone(), addr.downgrade());
        
        let count = sessions.len();
        println!(
            "✅ User {} (session {}) joined room. Total active users: {}",
            user_id, &session_id[..8], count
        );

        // 发送欢迎消息给新用户
        addr.do_send(ClientMessage(format!(
            "[SYSTEM] You joined room. Active users: {}",
            count
        )));

        // 通知房间内的其他用户
        let join_msg = format!("[SYSTEM] New user joined. Active users: {}", count);
        if let Some(sessions) = self.rooms.get(user_id) {
            for (sid, weak_addr) in sessions {
                if sid != &session_id {
                    if let Some(addr) = weak_addr.upgrade() {
                        addr.do_send(ClientMessage(join_msg.clone()));
                    }
                }
            }
        }
    }

    // 离开房间
    pub fn leave_room(&mut self, user_id: &str, session_id: &str) {
        let mut remaining = 0;
        let mut should_remove_room = false;
        
        if let Some(sessions) = self.rooms.get_mut(user_id) {
            sessions.remove(session_id);
            remaining = sessions.len();
            should_remove_room = sessions.is_empty();
        }
        
        if should_remove_room {
            self.rooms.remove(user_id);
            println!("🗑️ Room {} is now empty and removed", user_id);
        } else {
            println!(
                "👋 User {} (session {}) left room. Remaining users: {}",
                user_id, &session_id[..8], remaining
            );

            // 通知剩余用户
            let leave_msg = format!("[SYSTEM] User left. Remaining users: {}", remaining);
            if let Some(sessions) = self.rooms.get(user_id) {
                for (_, weak_addr) in sessions {
                    if let Some(addr) = weak_addr.upgrade() {
                        addr.do_send(ClientMessage(leave_msg.clone()));
                    }
                }
            }
        }
    }

    // 广播消息（排除指定 session）
    pub fn broadcast_to_room_excluding(
        &mut self,
        user_id: &str,
        message: String,
        exclude_session: Option<&str>,
    ) {
        // 先清理死亡连接
        self.cleanup_dead_connections(user_id);
        
        // 收集所有活跃的地址（避免借用冲突）
        let addresses: Vec<Addr<MyWs>> = if let Some(sessions) = self.rooms.get(user_id) {
            sessions
                .iter()
                .filter(|(session_id, _)| {
                    if let Some(exclude) = exclude_session {
                        session_id.as_str() != exclude
                    } else {
                        true
                    }
                })
                .filter_map(|(_, weak_addr)| weak_addr.upgrade())
                .collect()
        } else {
            Vec::new()
        };
        
        // 发送消息
        for addr in addresses {
            addr.do_send(ClientMessage(message.clone()));
        }
    }

    // 广播给所有人
    pub fn broadcast_to_room(&mut self, user_id: &str, message: String) {
        self.broadcast_to_room_excluding(user_id, message, None);
    }

    // 获取活跃用户数
    pub fn get_room_user_count(&mut self, user_id: &str) -> usize {
        self.cleanup_dead_connections(user_id);
        
        self.rooms
            .get(user_id)
            .map(|sessions| sessions.len())
            .unwrap_or(0)
    }

    // 调试信息
    pub fn debug_rooms(&mut self) {
        println!("=== DEBUG: Room Status ===");
        
        // 清理所有房间的死亡连接
        let user_ids: Vec<String> = self.rooms.keys().cloned().collect();
        for user_id in user_ids {
            self.cleanup_dead_connections(&user_id);
        }
        
        if self.rooms.is_empty() {
            println!("No active rooms");
        } else {
            for (user_id, sessions) in &self.rooms {
                println!("Room '{}': {} active session(s)", user_id, sessions.len());
            }
        }
        println!("==========================");
    }

    // 清理所有房间的死亡连接（定期任务用）
    pub fn cleanup_all_rooms(&mut self) {
        let user_ids: Vec<String> = self.rooms.keys().cloned().collect();
        for user_id in user_ids {
            self.cleanup_dead_connections(&user_id);
        }
    }
}

impl Actor for RoomManager {
    type Context = Context<Self>;
    
    fn started(&mut self, ctx: &mut Self::Context) {
        println!("🚀 RoomManager started");
        
        // 定期清理死亡连接（每30秒）
        ctx.run_interval(Duration::from_secs(30), |act, _| {
            println!("🧹 Running periodic cleanup...");
            act.cleanup_all_rooms();
        });
    }
}

// ============ 消息定义 ============

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage(pub String);

#[derive(Message)]
#[rtype(result = "()")]
pub struct JoinRoom {
    pub user_id: String,
    pub session_id: String,
    pub addr: Addr<MyWs>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LeaveRoom {
    pub user_id: String,
    pub session_id: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendToRoom {
    pub user_id: String,
    pub message: String,
    pub sender_session_id: String,
}

#[derive(Message)]
#[rtype(result = "usize")]
pub struct GetRoomUserCount {
    pub user_id: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct DebugRooms;

// ============ Handler 实现 ============

impl Handler<JoinRoom> for RoomManager {
    type Result = ();

    fn handle(&mut self, msg: JoinRoom, _: &mut Context<Self>) -> Self::Result {
        self.join_room(&msg.user_id, msg.session_id, msg.addr);
    }
}

impl Handler<LeaveRoom> for RoomManager {
    type Result = ();

    fn handle(&mut self, msg: LeaveRoom, _: &mut Context<Self>) -> Self::Result {
        self.leave_room(&msg.user_id, &msg.session_id);
    }
}

impl Handler<SendToRoom> for RoomManager {
    type Result = ();

    fn handle(&mut self, msg: SendToRoom, _: &mut Context<Self>) -> Self::Result {
        self.broadcast_to_room_excluding(
            &msg.user_id,
            msg.message,
            Some(&msg.sender_session_id),
        );
    }
}

impl Handler<GetRoomUserCount> for RoomManager {
    type Result = usize;

    fn handle(&mut self, msg: GetRoomUserCount, _: &mut Context<Self>) -> Self::Result {
        self.get_room_user_count(&msg.user_id)
    }
}

impl Handler<DebugRooms> for RoomManager {
    type Result = ();

    fn handle(&mut self, _: DebugRooms, _: &mut Context<Self>) -> Self::Result {
        self.debug_rooms();
    }
}

// ============ 心跳检测 ============

struct Heartbeat {
    last_heartbeat: Instant,
}

impl Heartbeat {
    fn new() -> Self {
        Self {
            last_heartbeat: Instant::now(),
        }
    }

    fn heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    fn is_alive(&self) -> bool {
        Instant::now().duration_since(self.last_heartbeat) < Duration::from_secs(30)
    }
}

// ============ WebSocket Actor ============

pub struct MyWs {
    user_id: String,
    room_manager: Addr<RoomManager>,
    heartbeat: Heartbeat,
    session_id: String,
}

impl MyWs {
    pub fn new(user_id: String, room_manager: Addr<RoomManager>) -> Self {
        Self {
            user_id,
            room_manager,
            heartbeat: Heartbeat::new(),
            session_id: Uuid::new_v4().to_string(),
        }
    }

    fn join_room(&self, ctx: &mut ws::WebsocketContext<Self>) {
        let addr = ctx.address();

        self.room_manager.do_send(JoinRoom {
            user_id: self.user_id.clone(),
            session_id: self.session_id.clone(),
            addr,
        });

        let welcome_msg = format!(
            "🚀 WELCOME: Connected as user {}\n\
            Session ID: {}\n\
            \n\
            📝 Commands: HELP | DEBUG | TEST\n\
            💬 Type any message to broadcast to your room.",
            self.user_id,
            &self.session_id[..8]
        );
        ctx.text(welcome_msg);
    }

    fn leave_room(&self) {
        self.room_manager.do_send(LeaveRoom {
            user_id: self.user_id.clone(),
            session_id: self.session_id.clone(),
        });
    }

    fn send_to_room(&self, message: String) {
        self.room_manager.do_send(SendToRoom {
            user_id: self.user_id.clone(),
            message,
            sender_session_id: self.session_id.clone(),
        });
    }
}

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!(
            "✅ WebSocket started for user: {} (session: {})",
            self.user_id, &self.session_id[..8]
        );

        self.join_room(ctx);

        // 心跳检测
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            if !act.heartbeat.is_alive() {
                println!("💔 Heartbeat failed for user: {} (session: {})", 
                    act.user_id, &act.session_id[..8]);
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        println!(
            "👋 WebSocket stopping for user: {} (session: {})",
            self.user_id, &self.session_id[..8]
        );

        self.leave_room();
        Running::Stop
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.heartbeat.heartbeat();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.heartbeat.heartbeat();
            }
            Ok(ws::Message::Text(text)) => {
                self.heartbeat.heartbeat();

                let message = text.trim();
                let timestamp = Local::now().format("%H:%M:%S").to_string();
                let session_short = &self.session_id[..8];

                // 发送给房间的其他人
                let room_msg = format!("[{}] {}: {}", timestamp, session_short, message);
                self.send_to_room(room_msg);

                // 给自己的回显
                let my_msg = format!("[You @ {}] {}", timestamp, message);
                ctx.text(my_msg);
            }
            Ok(ws::Message::Binary(bin)) => {
                self.heartbeat.heartbeat();
                ctx.binary(bin);
            }
            Ok(ws::Message::Close(reason)) => {
                println!(
                    "🔌 WebSocket closing for user {} (session: {}): {:?}",
                    self.user_id, &self.session_id[..8], reason
                );
                ctx.close(reason);
            }
            _ => (),
        }
    }
}

impl Handler<ClientMessage> for MyWs {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(msg.0);
    }
}

// ============ 应用状态 ============

#[derive(Clone)]
pub struct AppState {
    pub room_manager: Addr<RoomManager>,
}

impl AppState {
    pub fn new() -> Self {
        let room_manager = RoomManager::new().start();
        Self { room_manager }
    }
}