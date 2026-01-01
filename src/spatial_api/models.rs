use actix::prelude::*;
use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_web_actors::ws;
use chrono::Local;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use uuid::Uuid;

// 房间管理器 Actor
pub struct RoomManager {
    // user_id -> 该用户的所有连接地址
    rooms: HashMap<String, HashSet<Addr<MyWs>>>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    // 加入房间（基于user_id）
    pub fn join_room(&mut self, user_id: &str, addr: Addr<MyWs>) {
        let room = self
            .rooms
            .entry(user_id.to_string())
            .or_insert_with(HashSet::new);
        room.insert(addr.clone());

        let count = room.len();
        println!(
            "✅ User {} joined room. Total users in room: {}",
            user_id, count
        );

        // 发送欢迎消息给新加入的用户
        addr.do_send(ClientMessage(format!(
            "[SYSTEM] You joined room. Room users: {}",
            count
        )));

        // 通知房间内的其他用户有新成员加入
        self.broadcast_to_room_excluding(
            user_id,
            format!("[SYSTEM] New user joined. Room users: {}", count),
            Some(&addr),
        );
    }

    // 离开房间
    pub fn leave_room(&mut self, user_id: &str, addr: &Addr<MyWs>) {
        if let Some(clients) = self.rooms.get_mut(user_id) {
            clients.remove(addr);
            let remaining = clients.len();

            if clients.is_empty() {
                self.rooms.remove(user_id);
                println!("🗑️ Room {} is now empty and removed", user_id);
            } else {
                println!(
                    "👋 User left room {}. Remaining users: {}",
                    user_id, remaining
                );

                // 通知剩余用户有人离开
                self.broadcast_to_room_excluding(
                    user_id,
                    format!("[SYSTEM] User left. Remaining users: {}", remaining),
                    Some(addr),
                );
            }
        }
    }

    // 发送消息给指定user_id的房间（排除指定地址）
    pub fn broadcast_to_room_excluding(
        &self,
        user_id: &str,
        message: String,
        exclude_addr: Option<&Addr<MyWs>>,
    ) {
        if let Some(clients) = self.rooms.get(user_id) {
            for client in clients {
                if let Some(exclude) = exclude_addr {
                    if client == exclude {
                        continue;
                    }
                }

                // 发送消息到客户端
                client.do_send(ClientMessage(message.clone()));
            }
        }
    }

    // 发送消息给指定user_id的房间（包含所有人）
    pub fn broadcast_to_room(&self, user_id: &str, message: String) {
        if let Some(clients) = self.rooms.get(user_id) {
            for client in clients {
                client.do_send(ClientMessage(message.clone()));
            }
        }
    }

    // 获取房间内的用户数量
    pub fn get_room_user_count(&self, user_id: &str) -> usize {
        self.rooms
            .get(user_id)
            .map(|clients| clients.len())
            .unwrap_or(0)
    }

    // 调试：打印所有房间状态
    pub fn debug_rooms(&self) {
        println!("=== DEBUG: Room Status ===");
        if self.rooms.is_empty() {
            println!("No active rooms");
        }
        for (user_id, clients) in &self.rooms {
            println!("Room '{}': {} client(s)", user_id, clients.len());
        }
        println!("==========================");
    }
}

// Actor 实现
impl Actor for RoomManager {
    type Context = Context<Self>;
}

// 消息定义
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage(pub String);

#[derive(Message)]
#[rtype(result = "()")]
pub struct JoinRoom {
    pub user_id: String,
    pub addr: Addr<MyWs>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LeaveRoom {
    pub user_id: String,
    pub addr: Addr<MyWs>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SendToRoom {
    pub user_id: String,
    pub message: String,
    pub sender_addr: Addr<MyWs>, // 发送者的地址
}

#[derive(Message)]
#[rtype(result = "usize")]
pub struct GetRoomUserCount {
    pub user_id: String,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct DebugRooms;

// 处理 JoinRoom 消息
impl Handler<JoinRoom> for RoomManager {
    type Result = ();

    fn handle(&mut self, msg: JoinRoom, ctx: &mut Context<Self>) -> Self::Result {
        self.join_room(&msg.user_id, msg.addr);
    }
}

// 处理 LeaveRoom 消息
impl Handler<LeaveRoom> for RoomManager {
    type Result = ();

    fn handle(&mut self, msg: LeaveRoom, ctx: &mut Context<Self>) -> Self::Result {
        self.leave_room(&msg.user_id, &msg.addr);
    }
}

// 处理 SendToRoom 消息
impl Handler<SendToRoom> for RoomManager {
    type Result = ();

    fn handle(&mut self, msg: SendToRoom, ctx: &mut Context<Self>) -> Self::Result {
        // 广播给房间内的其他用户（排除发送者）
        self.broadcast_to_room_excluding(&msg.user_id, msg.message, Some(&msg.sender_addr));
    }
}

// 处理 GetRoomUserCount 消息
impl Handler<GetRoomUserCount> for RoomManager {
    type Result = usize;

    fn handle(&mut self, msg: GetRoomUserCount, ctx: &mut Context<Self>) -> Self::Result {
        self.get_room_user_count(&msg.user_id)
    }
}

// 处理 DebugRooms 消息
impl Handler<DebugRooms> for RoomManager {
    type Result = ();

    fn handle(&mut self, msg: DebugRooms, ctx: &mut Context<Self>) -> Self::Result {
        self.debug_rooms();
    }
}

// 心跳检测结构体
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

// MyWs 结构体
pub struct MyWs {
    user_id: String,
    room_manager: Addr<RoomManager>,
    heartbeat: Heartbeat,
    session_id: String,
    addr: Option<Addr<MyWs>>,
}

impl MyWs {
    pub fn new(user_id: String, room_manager: Addr<RoomManager>) -> Self {
        Self {
            user_id,
            room_manager,
            heartbeat: Heartbeat::new(),
            session_id: Uuid::new_v4().to_string(),
            addr: None,
        }
    }

    // 加入房间
    fn join_room(&mut self, ctx: &mut ws::WebsocketContext<Self>) {
        let addr = ctx.address();
        self.addr = Some(addr.clone());

        // 发送加入房间的消息
        self.room_manager.do_send(JoinRoom {
            user_id: self.user_id.clone(),
            addr: addr.clone(),
        });

        // 获取并显示房间信息
        let welcome_msg = format!(
            "🚀 WELCOME: Connected as user {}\n\
            Session ID: {}\n\
            You are in a room with other users who have the same user_id.\n\
            \n\
            📝 Available commands:\n\
            • HELP - Show this help message\n\
            • DEBUG - Show room status\n\
            • TEST - Send a test message\n\
            • LIST - List users in your room (coming soon)\n\
            \n\
            💬 Just type any message to broadcast to your room.",
            self.user_id,
            self.session_id.chars().take(8).collect::<String>()
        );
        ctx.text(welcome_msg);
    }

    // 离开房间
    fn leave_room(&mut self) {
        if let Some(addr) = &self.addr {
            self.room_manager.do_send(LeaveRoom {
                user_id: self.user_id.clone(),
                addr: addr.clone(),
            });
        }
    }

    // 发送消息到房间
    fn send_to_room(&self, message: String) {
        if let Some(addr) = &self.addr {
            self.room_manager.do_send(SendToRoom {
                user_id: self.user_id.clone(),
                message,
                sender_addr: addr.clone(),
            });
        }
    }
}

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!(
            "✅ WebSocket started for user: {} (session: {})",
            self.user_id, self.session_id
        );

        // 加入房间
        self.join_room(ctx);

        // 启动心跳检测
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            if !act.heartbeat.is_alive() {
                println!("💔 Heartbeat failed for user: {}", act.user_id);
                ctx.stop();
                return;
            }

            // 发送ping保持连接
            ctx.ping(b"");
        });
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        println!(
            "👋 WebSocket stopping for user: {} (session: {})",
            self.user_id, self.session_id
        );

        // 离开房间
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

                // 普通消息，发送给房间
                let timestamp = Local::now().format("%H:%M:%S").to_string();
                let session_short = self.session_id.chars().take(8).collect::<String>();

                // 发送给房间中的其他用户（排除自己）
                let room_msg = format!("[{}] {}: {}", timestamp, session_short, message);
                self.send_to_room(room_msg);

                // 给自己显示消息
                let my_msg = format!("[You @ {}] {}", timestamp, message);
                ctx.text(my_msg);
            }
            Ok(ws::Message::Binary(bin)) => {
                self.heartbeat.heartbeat();
                ctx.binary(bin);
            }
            Ok(ws::Message::Close(reason)) => {
                println!(
                    "🔌 WebSocket closing for user {}: {:?}",
                    self.user_id, reason
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
        // 接收来自房间管理器的消息
        ctx.text(msg.0);
    }
}

// 共享的应用程序状态
#[derive(Clone)]
pub struct AppState {
    pub room_manager: Addr<RoomManager>,
}

impl AppState {
    pub fn new() -> Self {
        // 启动房间管理器 Actor
        let room_manager = RoomManager::new().start();

        Self { room_manager }
    }
}
