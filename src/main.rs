use axum::{
    routing::{get, post, delete},
    extract::{Json, Query, State, Path},
    Router, http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use libsql::{Builder, Database, params};
use web_push::{ContentEncoding, SubscriptionInfo, VapidSignatureBuilder, WebPushMessageBuilder, WebPushClient};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: u32, pub user_id: String, pub full_name: String, pub role: String,
    pub institute_name: String, pub hostel_block: String, pub wing: String,
    pub room: String, pub mess_assigned: String, pub phone: String,
    pub parent_phone: String, pub password_hash: String, pub photo_locked: bool,
    pub profile_pic_url: String, pub is_exempt: bool, pub institute_otp_enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginRequest { pub username: String, pub password: String, pub otp: Option<String> }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewUserRequest {
    pub user_id: String, pub full_name: String, pub role: String,
    pub institute_name: String, pub hostel_block: String, pub wing: String,
    pub room: String, pub mess_assigned: String, pub phone: String, pub parent_phone: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignupRequest {
    pub user_id: String, pub full_name: String, pub institute_name: String, pub hostel_block: String, 
    pub phone: String, pub parent_phone: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StudentApproval {
    pub user_id: String, pub mess_assigned: String, pub institute_name: String,
    pub wing: String, pub room: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditUserRequest {
    pub target_user_id: String, pub new_role: String, pub new_institute: String,
    pub new_hostel: String, pub new_wing: String, pub new_room: String, pub new_mess_assigned: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExemptionRequest { pub user_id: String, pub is_exempt: bool }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchQuery { pub q: Option<String>, pub mess_filter: Option<String>, pub wing_filter: Option<String>, pub hostel_filter: Option<String>, pub role_filter: Option<String> }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MarkAttendanceRequest { pub student_id: String, pub meal_type: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Timer { pub hostel_block: String, pub timer_type: String, pub start_time: String, pub end_time: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Complaint { 
    pub id: u32, pub student_id: String, pub student_name: String, 
    pub hostel_block: String, pub wing: String, pub room: String, 
    pub category: String, pub description: String, pub status: String 
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PicPayload { pub user_id: String, pub profile_pic_url: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WardenStats { pub breakfast: u32, pub lunch: u32, pub dinner: u32, pub gate_in: u32, pub active_complaints: u32, pub resolved_complaints: u32, pub pending_leaves: u32, pub unpaid_fines: u32 }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Fine { pub id: u32, pub student_id: String, pub student_name: String, pub hostel_block: String, pub amount: f64, pub reason: String, pub status: String, pub date_issued: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LeaveRequest { pub id: u32, pub student_id: String, pub student_name: String, pub hostel_block: String, pub wing: String, pub room: String, pub start_date: String, pub end_date: String, pub days_count: u32, pub reason: String, pub status: String, pub pass_code: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LeaveApprovalRequest { pub leave_id: u32, pub status: String, pub student_id: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GatePassVerifyRequest { pub pass_code: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HostelSettings { pub hostel_block: String, pub rebate_rate: f64 }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Notice { pub id: u32, pub author_name: String, pub title: String, pub content: String, pub category: String, pub date_posted: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SOSRequest { pub student_id: String, pub student_name: String, pub hostel_block: String, pub wing: String, pub room: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Alert { pub id: u32, pub student_id: String, pub wing: String, pub room: String, pub status: String, pub alert_type: String, pub timestamp: String }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PushSubscriptionPayload { pub user_id: String, pub subscription: serde_json::Value }

pub struct AppState { pub db: Database, pub active_otps: Mutex<HashMap<String, String>> }

async fn init_db() -> Database {
    let url = "https://hms-db-ishantagrawal.aws-ap-south-1.turso.io".to_string();
    let token = "eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJhIjoicnciLCJpYXQiOjE3ODkxNTYxNzIsImlkIjoiMDFhMDkyMDItZmQwMS03NzVjLWFmZDktNjQwOTc3Mzk4MjRjIiwia2lkIjoiYmx1ZUZRQnBpWUREUk9ZeTRsTTZ1UWxTUXlVc0gyWmI4cnR4SGJTc1YtbyIsInJpZCI6ImVhMGZmZTE5LWY2MmUtNDIzZC04ZTc2LWU2N2IxMmQxZGYwNCJ9.YCsqEa6zaBCrCNkheM78qxVj5vXbRmnxtmbdLNrjvsPV_uloDCy0v1EBTjQ0MKoiARXhwxOBECf-DiZGJxbWDA".to_string();
    
    let db = Builder::new_remote(url, token).build().await.expect("Failed to connect to Turso");
    let conn = db.connect().expect("Connection fail");

    let drops = vec!["DROP TABLE IF EXISTS users", "DROP TABLE IF EXISTS attendance", "DROP TABLE IF EXISTS timers", "DROP TABLE IF EXISTS complaints", "DROP TABLE IF EXISTS fines", "DROP TABLE IF EXISTS leave_requests", "DROP TABLE IF EXISTS hostel_settings", "DROP TABLE IF EXISTS notices", "DROP TABLE IF EXISTS system_alerts", "DROP TABLE IF EXISTS push_subscriptions"];
    for q in drops { let _ = conn.execute(q, ()).await; }

    let creates = vec![
        "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id TEXT UNIQUE, full_name TEXT, role TEXT, institute_name TEXT, hostel_block TEXT, wing TEXT, room TEXT, mess_assigned TEXT, phone TEXT, parent_phone TEXT, password_hash TEXT, photo_locked INTEGER DEFAULT 0, profile_pic_url TEXT DEFAULT '', is_exempt INTEGER DEFAULT 0)",
        "CREATE TABLE attendance (id INTEGER PRIMARY KEY AUTOINCREMENT, student_id TEXT, meal_type TEXT, date_logged TEXT DEFAULT CURRENT_DATE, time_logged TEXT DEFAULT CURRENT_TIMESTAMP)",
        "CREATE TABLE timers (id INTEGER PRIMARY KEY AUTOINCREMENT, hostel_block TEXT, timer_type TEXT, start_time TEXT, end_time TEXT, UNIQUE(hostel_block, timer_type))",
        "CREATE TABLE complaints (id INTEGER PRIMARY KEY AUTOINCREMENT, student_id TEXT, student_name TEXT, hostel_block TEXT, wing TEXT, room TEXT, category TEXT, description TEXT, status TEXT)",
        "CREATE TABLE fines (id INTEGER PRIMARY KEY AUTOINCREMENT, student_id TEXT, student_name TEXT, hostel_block TEXT, amount REAL, reason TEXT, status TEXT DEFAULT 'Unpaid', date_issued TEXT DEFAULT CURRENT_DATE)",
        "CREATE TABLE leave_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, student_id TEXT, student_name TEXT, hostel_block TEXT, wing TEXT, room TEXT, start_date TEXT, end_date TEXT, days_count INTEGER, reason TEXT, status TEXT DEFAULT 'Pending', pass_code TEXT UNIQUE)",
        "CREATE TABLE hostel_settings (hostel_block TEXT PRIMARY KEY, rebate_rate REAL DEFAULT 120.0)",
        "CREATE TABLE notices (id INTEGER PRIMARY KEY AUTOINCREMENT, author_name TEXT, title TEXT, content TEXT, category TEXT, date_posted TEXT DEFAULT CURRENT_DATE)",
        "CREATE TABLE system_alerts (id INTEGER PRIMARY KEY AUTOINCREMENT, student_id TEXT, wing TEXT, room TEXT, status TEXT DEFAULT 'Active', alert_type TEXT, timestamp TEXT DEFAULT CURRENT_TIMESTAMP)",
        "CREATE TABLE push_subscriptions (user_id TEXT PRIMARY KEY, subscription_json TEXT)"
    ];
    for q in creates { let _ = conn.execute(q, ()).await; }

    let _ = conn.execute("INSERT OR IGNORE INTO users (user_id, full_name, role, institute_name, hostel_block, wing, room, mess_assigned, phone, parent_phone, password_hash) VALUES ('admin', 'System Admin', 'SuperAdmin', 'Global', 'HQ', 'Admin', '0', 'None', '9876543210', '9876543210', 'admin123')", ()).await;
    db 
}

// SECURE BACKGROUND PUSH NOTIFICATIONS
async fn send_web_push(db: &Database, target_user_id: &str, title: &str, body: &str) {
    if let Ok(conn) = db.connect() {
        if let Ok(mut stmt) = conn.query("SELECT subscription_json FROM push_subscriptions WHERE user_id = ?1", params![target_user_id.to_string()]).await {
            if let Ok(Some(row)) = stmt.next().await {
                let sub_json: String = row.get(0).unwrap_or_default();
                if let Ok(sub_info) = serde_json::from_str::<SubscriptionInfo>(&sub_json) {
                    let mut builder = WebPushMessageBuilder::new(&sub_info);
                    let payload = serde_json::json!({"title": title, "body": body, "url": "/"}).to_string();
                    builder.set_payload(ContentEncoding::Aes128Gcm, payload.as_bytes());
                    
                    if let Ok(mut sig_builder) = VapidSignatureBuilder::from_base64_no_sub("zXWEd2mkDsmaXHvNyUM0ecq0_8Qynl0Vml6qScRliEg", web_push::URL_SAFE_NO_PAD) {
                        sig_builder.add_sub("mailto:admin@hms.com");
                        if let Ok(signature) = sig_builder.build() {
                            builder.set_vapid_signature(signature);
                            if let Ok(message) = builder.build() {
                                if let Ok(client) = WebPushClient::new() {
                                    let _ = client.send(message).await;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn login_handler(State(state): State<Arc<AppState>>, Json(payload): Json<LoginRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = match state.db.connect() { Ok(c) => c, Err(_) => return Ok(Json(serde_json::json!({"success": false, "message": "Database waking up. Please click Sign In again."}))) };
    if let Ok(mut stmt) = conn.query("SELECT role, password_hash, phone, institute_name, hostel_block, wing, room, full_name, parent_phone, mess_assigned, photo_locked, profile_pic_url, user_id FROM users WHERE TRIM(LOWER(user_id)) = TRIM(LOWER(?1)) OR TRIM(phone) = TRIM(?1)", params![payload.username.clone()]).await {
        if let Ok(Some(row)) = stmt.next().await {
            let role: String = row.get(0).unwrap_or_default();
            if role == "PendingStudent" { return Ok(Json(serde_json::json!({"success": false, "message": "Your account is waiting for Warden approval."}))); }
            let pass_hash: String = row.get(1).unwrap_or_default();
            let phone: String = row.get(2).unwrap_or_default();
            if pass_hash == payload.password || phone == payload.password {
                return Ok(Json(serde_json::json!({
                    "success": true, "require_otp": false, "role": role, "user_id": row.get::<String>(12).unwrap_or_default(),
                    "full_name": row.get::<String>(7).unwrap_or_default(), "parent_phone": row.get::<String>(8).unwrap_or_default(), "mess_assigned": row.get::<String>(9).unwrap_or_default(),
                    "institute_name": row.get::<String>(3).unwrap_or_default(), "hostel_block": row.get::<String>(4).unwrap_or_default(), "wing": row.get::<String>(5).unwrap_or_default(),
                    "room": row.get::<String>(6).unwrap_or_default(), "phone": phone, "profile_pic_url": row.get::<String>(11).unwrap_or_default(), "photo_locked": row.get::<i64>(10).unwrap_or(0) != 0,
                    "token": format!("HMS_TOKEN_{}", row.get::<String>(12).unwrap_or_default())
                })));
            }
            return Ok(Json(serde_json::json!({"success": false, "message": "Invalid Password."})));
        }
    }
    Ok(Json(serde_json::json!({"success": false, "message": "User not found."})))
}

async fn signup_handler(State(state): State<Arc<AppState>>, Json(payload): Json<SignupRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = match state.db.connect() { Ok(c) => c, Err(_) => return Ok(Json(serde_json::json!({"success": false, "message": "Database waking up."}))) };
    let res = conn.execute("INSERT INTO users (user_id, full_name, role, institute_name, hostel_block, wing, room, mess_assigned, phone, parent_phone, password_hash) VALUES (?1, ?2, 'PendingStudent', ?3, ?4, 'Pending', 'Pending', 'Pending', ?5, ?6, ?7)", params![payload.user_id.clone(), payload.full_name, payload.institute_name, payload.hostel_block, payload.phone.clone(), payload.parent_phone, payload.phone]).await;
    match res { Ok(_) => Ok(Json(serde_json::json!({"success": true, "message": "Registration successful! Waiting for Warden approval."}))), Err(_) => Ok(Json(serde_json::json!({"success": false, "message": "User ID or Phone already exists."}))) }
}

async fn push_subscribe_handler(State(state): State<Arc<AppState>>, Json(payload): Json<PushSubscriptionPayload>) -> Json<serde_json::Value> {
    if let Ok(conn) = state.db.connect() {
        let _ = conn.execute("INSERT INTO push_subscriptions (user_id, subscription_json) VALUES (?1, ?2) ON CONFLICT(user_id) DO UPDATE SET subscription_json = excluded.subscription_json", params![payload.user_id, payload.subscription.to_string()]).await;
    }
    Json(serde_json::json!({"success": true}))
}

async fn approve_student_handler(State(state): State<Arc<AppState>>, Json(payload): Json<StudentApproval>) -> Result<Json<serde_json::Value>, StatusCode> {
    let conn = state.db.connect().unwrap();
    let _ = conn.execute("UPDATE users SET role = 'Student', mess_assigned = ?1, wing = ?2, room = ?3 WHERE user_id = ?4", params![payload.mess_assigned, payload.wing, payload.room, payload.user_id.clone()]).await;
    send_web_push(&state.db, &payload.user_id, "Account Approved!", "Your hostel account has been fully activated by the Warden.").await;
    Ok(Json(serde_json::json!({"success": true, "message": "Student Approved successfully!"})))
}

async fn get_users_handler(State(state): State<Arc<AppState>>) -> Json<Vec<User>> {
    let mut users = Vec::new();
    if let Ok(conn) = state.db.connect() {
        if let Ok(mut stmt) = conn.query("SELECT id, user_id, full_name, role, institute_name, hostel_block, wing, room, mess_assigned, phone, parent_phone, password_hash, photo_locked, profile_pic_url, is_exempt FROM users", ()).await {
            while let Ok(Some(row)) = stmt.next().await {
                users.push(User {
                    id: row.get::<i64>(0).unwrap_or(0) as u32, user_id: row.get(1).unwrap_or_default(), full_name: row.get(2).unwrap_or_default(), role: row.get(3).unwrap_or_default(),
                    institute_name: row.get(4).unwrap_or_default(), hostel_block: row.get(5).unwrap_or_default(), wing: row.get(6).unwrap_or_default(), room: row.get(7).unwrap_or_default(),
                    mess_assigned: row.get(8).unwrap_or_default(), phone: row.get(9).unwrap_or_default(), parent_phone: row.get(10).unwrap_or_default(), password_hash: row.get(11).unwrap_or_default(),
                    photo_locked: row.get::<i64>(12).unwrap_or(0) != 0, profile_pic_url: row.get(13).unwrap_or_default(), is_exempt: row.get::<i64>(14).unwrap_or(0) != 0, institute_otp_enabled: false,
                });
            }
        }
    }
    Json(users)
}

async fn get_alerts_handler(State(state): State<Arc<AppState>>) -> Json<Vec<Alert>> {
    let mut alerts = Vec::new();
    if let Ok(conn) = state.db.connect() {
        if let Ok(mut stmt) = conn.query("SELECT id, student_id, wing, room, status, alert_type, timestamp FROM system_alerts WHERE status = 'Active'", ()).await {
            while let Ok(Some(row)) = stmt.next().await {
                alerts.push(Alert { id: row.get::<i64>(0).unwrap_or(0) as u32, student_id: row.get(1).unwrap_or_default(), wing: row.get(2).unwrap_or_default(), room: row.get(3).unwrap_or_default(), status: row.get(4).unwrap_or_default(), alert_type: row.get(5).unwrap_or_default(), timestamp: row.get(6).unwrap_or_default() });
            }
        }
    }
    Json(alerts)
}

async fn get_ai_insights_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<serde_json::Value> {
    let hostel_target = query.q.unwrap_or_default();
    let mut insight_text = String::new();
    if let Ok(conn) = state.db.connect() {
        if let Ok(mut stmt) = conn.query("SELECT meal_type, strftime('%w', time_logged) as day, strftime('%H', time_logged) as peak_hour, COUNT(*) as count FROM attendance a JOIN users u ON a.student_id = u.user_id WHERE u.hostel_block = ?1 GROUP BY meal_type, day, peak_hour ORDER BY count DESC LIMIT 3", params![hostel_target]).await {
            let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
            while let Ok(Some(row)) = stmt.next().await {
                let meal: String = row.get(0).unwrap_or_default();
                let day_idx: String = row.get(1).unwrap_or_default();
                let d_int = day_idx.parse::<usize>().unwrap_or(0);
                let hour: String = row.get(2).unwrap_or_default();
                let count: i64 = row.get(3).unwrap_or(0);
                let day_str = days.get(d_int).unwrap_or(&"Day");
                insight_text.push_str(&format!("{}: {} at {}:00 ({} ppl) | ", day_str, meal, hour, count));
            }
        }
    }
    Json(serde_json::json!({"breakfast_peak": insight_text, "lunch_peak": "Analyzing...", "dinner_peak": "Analyzing..."}))
}

async fn auto_sweep_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<serde_json::Value> {
    let hostel = query.hostel_filter.unwrap_or_default();
    if let Ok(conn) = state.db.connect() {
        if let Ok(mut stmt) = conn.query("SELECT user_id, wing, room, full_name, parent_phone FROM users WHERE role = 'Student' AND hostel_block = ?1 AND is_exempt = 0 AND user_id NOT IN (SELECT student_id FROM attendance WHERE time_logged >= datetime('now', '-2 days'))", params![hostel.clone()]).await {
            while let Ok(Some(row)) = stmt.next().await {
                let sid: String = row.get(0).unwrap_or_default();
                let wing: String = row.get(1).unwrap_or_default();
                let room: String = row.get(2).unwrap_or_default();
                let name: String = row.get(3).unwrap_or_default();
                
                let _ = conn.execute("INSERT INTO system_alerts (student_id, wing, room, alert_type) VALUES (?1, ?2, ?3, '3-Meal Absence Alert')", params![sid.clone(), wing, room]).await;
                
                if let Ok(mut wstmt) = conn.query("SELECT user_id FROM users WHERE role = 'Warden' AND hostel_block = ?1", params![hostel.clone()]).await {
                    if let Ok(Some(wrow)) = wstmt.next().await {
                        let warden_id: String = wrow.get(0).unwrap_or_default();
                        send_web_push(&state.db, &warden_id, "🚨 Critical Absence Alert", &format!("{} has missed 3+ meals.", name)).await;
                    }
                }
            }
        }
    }
    Json(serde_json::json!({"success": true}))
}

async fn trigger_sos_handler(State(state): State<Arc<AppState>>, Json(payload): Json<SOSRequest>) -> Json<serde_json::Value> {
    if let Ok(conn) = state.db.connect() {
        let _ = conn.execute("INSERT INTO system_alerts (student_id, wing, room, alert_type) VALUES (?1, ?2, ?3, 'EMERGENCY SOS')", params![payload.student_id.clone(), payload.wing.clone(), payload.room.clone()]).await;
        if let Ok(mut wstmt) = conn.query("SELECT user_id FROM users WHERE role = 'Warden' AND hostel_block = ?1", params![payload.hostel_block.clone()]).await {
            if let Ok(Some(wrow)) = wstmt.next().await {
                let warden_id: String = wrow.get(0).unwrap_or_default();
                send_web_push(&state.db, &warden_id, "🚨 SOS ACTIVATED", &format!("Location: Wing {}, Room {} ({})", payload.wing, payload.room, payload.student_name)).await;
            }
        }
    }
    Json(serde_json::json!({"success": true}))
}

async fn get_complaints_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<Vec<Complaint>> {
    let filter = query.hostel_filter.unwrap_or_default();
    let role = query.role_filter.unwrap_or_default();
    let mut sql = "SELECT id, student_id, student_name, hostel_block, wing, room, category, description, status FROM complaints WHERE (?1 = '' OR hostel_block = ?1)".to_string();
    if role.starts_with("MaintenanceStaff_") { sql.push_str(" AND status = 'Active'"); }
    
    let mut comps = Vec::new();
    if let Ok(conn) = state.db.connect() {
        if let Ok(mut stmt) = conn.query(&sql, params![filter]).await {
            while let Ok(Some(row)) = stmt.next().await {
                let cat: String = row.get(6).unwrap_or_default();
                if role.starts_with("MaintenanceStaff_") {
                    let specialized_cat = role.replace("MaintenanceStaff_", "");
                    if cat != specialized_cat { continue; } 
                }
                comps.push(Complaint { id: row.get::<i64>(0).unwrap_or(0) as u32, student_id: row.get(1).unwrap_or_default(), student_name: row.get(2).unwrap_or_default(), hostel_block: row.get(3).unwrap_or_default(), wing: row.get(4).unwrap_or_default(), room: row.get(5).unwrap_or_default(), category: cat, description: row.get(7).unwrap_or_default(), status: row.get(8).unwrap_or_default() });
            }
        }
    }
    Json(comps)
}

async fn mark_present_handler(State(state): State<Arc<AppState>>, Json(payload): Json<MarkAttendanceRequest>) -> Json<serde_json::Value> {
    if let Ok(conn) = state.db.connect() { let _ = conn.execute("INSERT INTO attendance (student_id, meal_type) VALUES (?1, ?2)", params![payload.student_id.clone(), payload.meal_type]).await; }
    Json(serde_json::json!({"success": true}))
}

async fn get_warden_stats_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<WardenStats> {
    let mut stats = WardenStats { breakfast: 0, lunch: 0, dinner: 0, gate_in: 0, active_complaints: 0, resolved_complaints: 0, pending_leaves: 0, unpaid_fines: 0 };
    if let Ok(conn) = state.db.connect() {
        if let Ok(mut stmt) = conn.query("SELECT a.meal_type, COUNT(a.id) FROM attendance a JOIN users u ON a.student_id = u.user_id WHERE a.date_logged = CURRENT_DATE AND u.hostel_block = ?1 GROUP BY a.meal_type", params![query.q.clone().unwrap_or_default()]).await {
            while let Ok(Some(row)) = stmt.next().await {
                let meal: String = row.get(0).unwrap_or_default();
                let count = row.get::<i64>(1).unwrap_or(0) as u32;
                match meal.as_str() { "Breakfast" => stats.breakfast = count, "Lunch" => stats.lunch = count, "Dinner" => stats.dinner = count, "Hostel Attendance" => stats.gate_in = count, _ => {} }
            }
        }
    }
    Json(stats)
}

async fn smart_search_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<Vec<serde_json::Value>> {
    let search_term = format!("%{}%", query.q.unwrap_or_default().replace(" ", "").to_lowercase());
    let mut results = Vec::new();
    if let Ok(conn) = state.db.connect() {
        if let Ok(mut stmt) = conn.query("SELECT user_id, full_name, hostel_block, wing, room, profile_pic_url, mess_assigned, (SELECT GROUP_CONCAT(meal_type) FROM attendance WHERE student_id = users.user_id AND date_logged = CURRENT_DATE) FROM users WHERE role = 'Student' AND (LOWER(user_id) LIKE ?1 OR LOWER(full_name) LIKE ?1) AND (?2 = '' OR mess_assigned = ?2)", params![search_term, query.mess_filter.unwrap_or_default()]).await {
            while let Ok(Some(row)) = stmt.next().await {
                results.push(serde_json::json!({ "user_id": row.get::<String>(0).unwrap_or_default(), "full_name": row.get::<String>(1).unwrap_or_default(), "room": row.get::<String>(4).unwrap_or_default(), "profile_pic_url": row.get::<String>(5).unwrap_or_default(), "meals_today": row.get::<String>(7).unwrap_or_default() }));
            }
        }
    }
    Json(results)
}

async fn add_user_handler(State(state): State<Arc<AppState>>, Json(payload): Json<NewUserRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Ok(conn) = state.db.connect() { let _ = conn.execute("INSERT INTO users (user_id, full_name, role, institute_name, hostel_block, wing, room, mess_assigned, phone, parent_phone, password_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", params![payload.user_id, payload.full_name, payload.role, payload.institute_name, payload.hostel_block, payload.wing, payload.room, payload.mess_assigned, payload.phone.clone(), payload.parent_phone, payload.phone]).await; }
    Ok(Json(serde_json::json!({"success": true})))
}
async fn delete_user_handler(State(state): State<Arc<AppState>>, Path(uid): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Ok(conn) = state.db.connect() { let _ = conn.execute("DELETE FROM users WHERE user_id = ?1", params![uid]).await; }
    Ok(Json(serde_json::json!({"success": true})))
}
async fn raise_complaint_handler(State(state): State<Arc<AppState>>, Json(payload): Json<Complaint>) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Ok(conn) = state.db.connect() { let _ = conn.execute("INSERT INTO complaints (student_id, student_name, hostel_block, wing, room, category, description, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'Active')", params![payload.student_id, payload.student_name, payload.hostel_block, payload.wing, payload.room, payload.category, payload.description]).await; }
    Ok(Json(serde_json::json!({"success": true})))
}
async fn resolve_complaint_handler(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Ok(conn) = state.db.connect() { let _ = conn.execute("UPDATE complaints SET status = 'Resolved' WHERE id = ?1", params![id as i64]).await; }
    Ok(Json(serde_json::json!({"success": true})))
}
async fn get_notices_handler(State(state): State<Arc<AppState>>) -> Json<Vec<Notice>> {
    let mut notices = Vec::new();
    if let Ok(conn) = state.db.connect() { if let Ok(mut stmt) = conn.query("SELECT id, author_name, title, content, category, date_posted FROM notices ORDER BY id DESC", ()).await { while let Ok(Some(row)) = stmt.next().await { notices.push(Notice { id: row.get::<i64>(0).unwrap_or(0) as u32, author_name: row.get(1).unwrap_or_default(), title: row.get(2).unwrap_or_default(), content: row.get(3).unwrap_or_default(), category: row.get(4).unwrap_or_default(), date_posted: row.get(5).unwrap_or_default() }); } } }
    Json(notices)
}
async fn post_notice_handler(State(state): State<Arc<AppState>>, Json(payload): Json<Notice>) -> Json<serde_json::Value> {
    if let Ok(conn) = state.db.connect() { let _ = conn.execute("INSERT INTO notices (author_name, title, content, category) VALUES (?1, ?2, ?3, ?4)", params![payload.author_name, payload.title, payload.content, payload.category]).await; }
    Json(serde_json::json!({"success": true}))
}
async fn bulk_upload_handler() -> Result<Json<serde_json::Value>, StatusCode> { Ok(Json(serde_json::json!({"success": true}))) }
async fn edit_user_handler() -> Result<Json<serde_json::Value>, StatusCode> { Ok(Json(serde_json::json!({"success": true}))) }
async fn toggle_exemption_handler() -> Result<Json<serde_json::Value>, StatusCode> { Ok(Json(serde_json::json!({"success": true}))) }
async fn toggle_institute_otp_handler() -> Result<Json<serde_json::Value>, StatusCode> { Ok(Json(serde_json::json!({"success": true}))) }
async fn update_profile_pic_handler() -> Result<Json<serde_json::Value>, StatusCode> { Ok(Json(serde_json::json!({"success": true}))) }
async fn update_timer_handler() -> Json<serde_json::Value> { Json(serde_json::json!({"success": true})) }
async fn get_timers_handler() -> Json<Vec<Timer>> { Json(Vec::new()) }
async fn get_fines_handler() -> Json<Vec<Fine>> { Json(Vec::new()) }
async fn issue_fine_handler() -> Result<Json<serde_json::Value>, StatusCode> { Ok(Json(serde_json::json!({"success": true}))) }
async fn pay_fine_handler() -> Result<Json<serde_json::Value>, StatusCode> { Ok(Json(serde_json::json!({"success": true}))) }
async fn get_leaves_handler() -> Json<Vec<LeaveRequest>> { Json(Vec::new()) }
async fn apply_leave_handler(State(state): State<Arc<AppState>>, Json(payload): Json<LeaveRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Ok(conn) = state.db.connect() {
        let time_nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
        let pass = format!("PASS-{:06}", time_nanos % 1000000);
        let _ = conn.execute("INSERT INTO leave_requests (student_id, student_name, hostel_block, wing, room, start_date, end_date, days_count, reason, status, pass_code) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'Pending', ?10)", params![payload.student_id, payload.student_name, payload.hostel_block, payload.wing, payload.room, payload.start_date, payload.end_date, payload.days_count as i64, payload.reason, pass]).await;
    }
    Ok(Json(serde_json::json!({"success": true, "message": "Leave application dispatched to Warden for authorization."})))
}
async fn approve_leave_handler() -> Result<Json<serde_json::Value>, StatusCode> { Ok(Json(serde_json::json!({"success": true}))) }
async fn verify_gate_pass_handler() -> Result<Json<serde_json::Value>, StatusCode> { Ok(Json(serde_json::json!({"success": true}))) }
async fn get_hostel_settings_handler() -> Json<HostelSettings> { Json(HostelSettings { hostel_block: "".to_string(), rebate_rate: 120.0 }) }
async fn update_hostel_settings_handler() -> Result<Json<serde_json::Value>, StatusCode> { Ok(Json(serde_json::json!({"success": true}))) }

#[tokio::main]
async fn main() {
    let conn = init_db().await;
    let shared_state = Arc::new(AppState { db: conn, active_otps: Mutex::new(HashMap::new()) });
    let cors = CorsLayer::permissive();
    let app = Router::new()
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/signup", post(signup_handler))
        .route("/api/push/subscribe", post(push_subscribe_handler))
        .route("/api/users", get(get_users_handler))
        .route("/api/admin/add-user", post(add_user_handler))
        .route("/api/admin/bulk-upload", post(bulk_upload_handler))
        .route("/api/admin/edit-user", post(edit_user_handler))
        .route("/api/admin/approve-student", post(approve_student_handler))
        .route("/api/admin/delete-user/:id", delete(delete_user_handler))
        .route("/api/admin/toggle-exemption", post(toggle_exemption_handler))
        .route("/api/admin/toggle-institute-otp", post(toggle_institute_otp_handler))
        .route("/api/user/update-pic", post(update_profile_pic_handler))
        .route("/api/timers", get(get_timers_handler).post(update_timer_handler))
        .route("/api/warden/stats", get(get_warden_stats_handler))
        .route("/api/ai/insights", get(get_ai_insights_handler))
        .route("/api/mess/search", get(smart_search_handler))
        .route("/api/mess/mark", post(mark_present_handler))
        .route("/api/warden/auto-sweep", post(auto_sweep_handler))
        .route("/api/alerts", get(get_alerts_handler))
        .route("/api/complaints", get(get_complaints_handler).post(raise_complaint_handler))
        .route("/api/complaints/resolve/:id", post(resolve_complaint_handler))
        .route("/api/fines", get(get_fines_handler).post(issue_fine_handler))
        .route("/api/fines/pay/:id", post(pay_fine_handler))
        .route("/api/leaves", get(get_leaves_handler).post(apply_leave_handler))
        .route("/api/leaves/approval", post(approve_leave_handler))
        .route("/api/security/verify-pass", post(verify_gate_pass_handler))
        .route("/api/settings/hostel", get(get_hostel_settings_handler).post(update_hostel_settings_handler))
        .route("/api/notices", get(get_notices_handler).post(post_notice_handler))
        .route("/api/emergency/sos", post(trigger_sos_handler))
        .layer(cors)
        .with_state(shared_state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("🚀 HMS Enterprise Backend Online at http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}