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
use rusqlite::{Connection, params};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: u32, pub user_id: String, pub full_name: String, pub role: String,
    pub institute_name: String, pub hostel_block: String, pub wing: String,
    pub room: String, pub mess_assigned: String, pub phone: String,
    pub parent_phone: String, pub password_hash: String, pub photo_locked: bool,
    pub profile_pic_url: String, pub is_exempt: bool,
    pub institute_otp_enabled: bool,
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
pub struct EditUserRequest {
    pub target_user_id: String, pub new_role: String, pub new_institute: String,
    pub new_hostel: String, pub new_wing: String, pub new_room: String, pub new_mess_assigned: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExemptionRequest { pub user_id: String, pub is_exempt: bool }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstituteOtpToggle { pub institute_name: String, pub otp_enabled: bool }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchQuery { 
    pub q: Option<String>, pub mess_filter: Option<String>, pub wing_filter: Option<String>, pub hostel_filter: Option<String>, pub role_filter: Option<String>
}

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
pub struct WardenStats {
    pub breakfast: u32, pub lunch: u32, pub dinner: u32, pub gate_in: u32,
    pub active_complaints: u32, pub resolved_complaints: u32,
    pub pending_leaves: u32, pub unpaid_fines: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Fine {
    pub id: u32, pub student_id: String, pub student_name: String,
    pub hostel_block: String, pub amount: f64, pub reason: String,
    pub status: String, pub date_issued: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LeaveRequest {
    pub id: u32, pub student_id: String, pub student_name: String,
    pub hostel_block: String, pub wing: String, pub room: String,
    pub start_date: String, pub end_date: String, pub days_count: u32,
    pub reason: String, pub status: String, pub pass_code: String,
}

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

pub struct AppState {
    pub db: Mutex<Connection>,
    pub active_otps: Mutex<HashMap<String, String>>, 
}

fn init_db() -> Connection {
    let conn = Connection::open("erp_production.db").expect("Failed to open SQLite DB");
    
    conn.busy_timeout(std::time::Duration::from_secs(5)).unwrap();
    conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();

    conn.execute("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id TEXT UNIQUE, full_name TEXT, role TEXT, institute_name TEXT, hostel_block TEXT, wing TEXT, room TEXT, mess_assigned TEXT, phone TEXT, parent_phone TEXT, password_hash TEXT, photo_locked BOOLEAN DEFAULT 0, profile_pic_url TEXT DEFAULT '', is_exempt BOOLEAN DEFAULT 0)", []).unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS attendance (id INTEGER PRIMARY KEY AUTOINCREMENT, student_id TEXT, meal_type TEXT, date_logged TEXT DEFAULT CURRENT_DATE, time_logged TEXT DEFAULT CURRENT_TIMESTAMP)", []).unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS timers (id INTEGER PRIMARY KEY AUTOINCREMENT, hostel_block TEXT, timer_type TEXT, start_time TEXT, end_time TEXT, UNIQUE(hostel_block, timer_type))", []).unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS complaints (id INTEGER PRIMARY KEY AUTOINCREMENT, student_id TEXT, student_name TEXT, hostel_block TEXT, wing TEXT, room TEXT, category TEXT, description TEXT, status TEXT)", []).unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS institute_settings (institute_name TEXT PRIMARY KEY, otp_enabled INTEGER DEFAULT 1)", []).unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS fines (id INTEGER PRIMARY KEY AUTOINCREMENT, student_id TEXT, student_name TEXT, hostel_block TEXT, amount REAL, reason TEXT, status TEXT DEFAULT 'Unpaid', date_issued TEXT DEFAULT CURRENT_DATE)", []).unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS leave_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, student_id TEXT, student_name TEXT, hostel_block TEXT, wing TEXT, room TEXT, start_date TEXT, end_date TEXT, days_count INTEGER, reason TEXT, status TEXT DEFAULT 'Pending', pass_code TEXT UNIQUE)", []).unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS hostel_settings (hostel_block TEXT PRIMARY KEY, rebate_rate REAL DEFAULT 120.0)", []).unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS notices (id INTEGER PRIMARY KEY AUTOINCREMENT, author_name TEXT, title TEXT, content TEXT, category TEXT, date_posted TEXT DEFAULT CURRENT_DATE)", []).unwrap();
    conn.execute("CREATE TABLE IF NOT EXISTS sos_alerts (id INTEGER PRIMARY KEY AUTOINCREMENT, student_id TEXT, wing TEXT, room TEXT, status TEXT DEFAULT 'Active', timestamp TEXT DEFAULT CURRENT_TIMESTAMP)", []).unwrap();

    let _ = conn.execute(
        "INSERT OR IGNORE INTO users (user_id, full_name, role, institute_name, hostel_block, wing, room, mess_assigned, phone, parent_phone, password_hash, photo_locked, profile_pic_url, is_exempt) 
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params!["Ishant Agarwal", "Ishant Agarwal", "SuperAdmin", "Global Network", "HQ", "Admin", "Master", "None", "9876543210", "9876543210", "Ishant@1077", 1, "https://api.dicebear.com/7.x/avataaars/svg?seed=Ishant", 0],
    );
    conn
}

async fn login_handler(State(state): State<Arc<AppState>>, Json(payload): Json<LoginRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT u.role, u.password_hash, u.phone, u.institute_name, u.hostel_block, u.wing, u.room, u.full_name, u.parent_phone, u.mess_assigned, u.photo_locked, u.profile_pic_url, COALESCE(s.otp_enabled, 1) 
         FROM users u LEFT JOIN institute_settings s ON u.institute_name = s.institute_name 
         WHERE TRIM(LOWER(u.user_id)) = TRIM(LOWER(?1))"
    ).unwrap();
    
    let result = stmt.query_row(params![payload.username], |row| Ok((
        row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, 
        row.get::<_, String>(3).unwrap_or_default(), row.get::<_, String>(4).unwrap_or_default(), 
        row.get::<_, String>(5).unwrap_or_default(), row.get::<_, String>(6).unwrap_or_default(), 
        row.get::<_, String>(7).unwrap_or_default(), row.get::<_, String>(8).unwrap_or_default(), 
        row.get::<_, String>(9).unwrap_or_default(), row.get::<_, bool>(10).unwrap_or(false), 
        row.get::<_, String>(11).unwrap_or_default(), row.get::<_, i32>(12).unwrap_or(1)
    )));

    match result {
        Ok((role, pass_hash, phone, institute, hostel, wing, room, full_name, parent_phone, mess, locked, pic, otp_int)) => {
            if pass_hash == payload.password || phone == payload.password {
                let user_key = payload.username.to_lowercase().trim().to_string();
                let otp_enabled = otp_int != 0; 
                let require_otp = role != "SuperAdmin" && otp_enabled;

                if payload.otp.is_none() && require_otp {
                    let time_nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
                    let generated_otp = format!("{:06}", time_nanos % 1000000);
                    state.active_otps.lock().unwrap().insert(user_key, generated_otp.clone());
                    
                    println!("========================================");
                    println!("🟢 [WHATSAPP BUSINESS API SIMULATION]");
                    println!("To Phone: {}", phone);
                    println!("Message: Your HMS Portal Login OTP is: {}", generated_otp);
                    println!("========================================");

                    return Ok(Json(serde_json::json!({"success": true, "require_otp": true, "message": format!("WhatsApp OTP sent to {}.", phone)})));
                }

                let is_valid_otp = if !require_otp { true } else {
                    let map = state.active_otps.lock().unwrap();
                    let stored_otp = map.get(&user_key);
                    stored_otp == payload.otp.as_ref()
                };

                if is_valid_otp {
                    state.active_otps.lock().unwrap().remove(&user_key);
                    return Ok(Json(serde_json::json!({
                        "success": true, "require_otp": false, "role": role, "user_id": payload.username,
                        "full_name": full_name, "parent_phone": parent_phone, "mess_assigned": mess,
                        "institute_name": institute, "hostel_block": hostel, "wing": wing,
                        "room": room, "phone": phone, "profile_pic_url": pic, "photo_locked": locked,
                        "token": format!("HMS_TOKEN_{}", payload.username)
                    })));
                } else {
                    return Ok(Json(serde_json::json!({"success": false, "message": "Incorrect OTP."})));
                }
            }
            Ok(Json(serde_json::json!({"success": false, "message": "Invalid Username or Password."})))
        }
        _ => Ok(Json(serde_json::json!({"success": false, "message": "User not found."}))),
    }
}

async fn get_hostel_settings_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<HostelSettings> {
    let db = state.db.lock().unwrap();
    let hostel = query.q.unwrap_or_default();
    let rate: f64 = db.query_row("SELECT rebate_rate FROM hostel_settings WHERE hostel_block = ?1", params![hostel], |r| r.get(0)).unwrap_or(120.0);
    Json(HostelSettings { hostel_block: hostel, rebate_rate: rate })
}

async fn update_hostel_settings_handler(State(state): State<Arc<AppState>>, Json(payload): Json<HostelSettings>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    db.execute(
        "INSERT INTO hostel_settings (hostel_block, rebate_rate) VALUES (?1, ?2) ON CONFLICT(hostel_block) DO UPDATE SET rebate_rate=excluded.rebate_rate",
        params![payload.hostel_block, payload.rebate_rate],
    ).unwrap();
    Ok(Json(serde_json::json!({"success": true, "message": format!("Rebate rate successfully locked to ₹{}/day.", payload.rebate_rate)})))
}

async fn get_ai_insights_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap();
    let hostel_target = query.q.unwrap_or_default();
    
    let mut stmt = db.prepare("SELECT a.meal_type, strftime('%H', a.time_logged) as peak_hour, COUNT(*) as count FROM attendance a JOIN users u ON a.student_id = u.user_id WHERE u.hostel_block = ?1 GROUP BY a.meal_type, peak_hour ORDER BY count DESC").unwrap();
    let mut insights = HashMap::new();
    let iter = stmt.query_map(params![hostel_target], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, u32>(2)?))).unwrap();

    for row in iter.flatten() {
        let (meal, hour, count) = row;
        if !insights.contains_key(&meal) {
            let hour_num: u32 = hour.parse().unwrap_or(0);
            let time_window = format!("{:02}:00 - {:02}:59", hour_num, hour_num);
            insights.insert(meal, format!("{} ({} students)", time_window, count));
        }
    }

    Json(serde_json::json!({
        "breakfast_peak": insights.get("Breakfast").unwrap_or(&"Not enough data".to_string()), 
        "lunch_peak": insights.get("Lunch").unwrap_or(&"Not enough data".to_string()), 
        "dinner_peak": insights.get("Dinner").unwrap_or(&"Not enough data".to_string())
    }))
}

async fn get_warden_stats_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<WardenStats> {
    let db = state.db.lock().unwrap();
    let hostel_target = query.q.unwrap_or_default();
    let mut stats = WardenStats { 
        breakfast: 0, lunch: 0, dinner: 0, gate_in: 0, 
        active_complaints: 0, resolved_complaints: 0,
        pending_leaves: 0, unpaid_fines: 0,
    };

    let mut stmt_att = db.prepare("SELECT a.meal_type, COUNT(a.id) FROM attendance a JOIN users u ON a.student_id = u.user_id WHERE a.date_logged = CURRENT_DATE AND u.hostel_block = ?1 GROUP BY a.meal_type").unwrap();
    let iter_att = stmt_att.query_map(params![hostel_target], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))).unwrap();
    for row in iter_att.flatten() { match row.0.as_str() { "Breakfast" => stats.breakfast = row.1, "Lunch" => stats.lunch = row.1, "Dinner" => stats.dinner = row.1, "Hostel Attendance" => stats.gate_in = row.1, _ => {} } }

    let mut stmt_comp = db.prepare("SELECT status, COUNT(id) FROM complaints WHERE hostel_block = ?1 GROUP BY status").unwrap();
    let iter_comp = stmt_comp.query_map(params![hostel_target], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))).unwrap();
    for row in iter_comp.flatten() { if row.0 == "Active" { stats.active_complaints = row.1; } else if row.0 == "Resolved" { stats.resolved_complaints = row.1; } }

    stats.pending_leaves = db.query_row("SELECT COUNT(id) FROM leave_requests WHERE hostel_block = ?1 AND status = 'Pending'", params![hostel_target], |r| r.get(0)).unwrap_or(0);
    stats.unpaid_fines = db.query_row("SELECT COUNT(id) FROM fines WHERE hostel_block = ?1 AND status = 'Unpaid'", params![hostel_target], |r| r.get(0)).unwrap_or(0);

    Json(stats)
}

async fn get_users_handler(State(state): State<Arc<AppState>>) -> Json<Vec<User>> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare(
        "SELECT u.id, u.user_id, u.full_name, u.role, u.institute_name, u.hostel_block, u.wing, u.room, u.mess_assigned, u.phone, u.parent_phone, u.password_hash, u.photo_locked, u.profile_pic_url, u.is_exempt, COALESCE(s.otp_enabled, 1) 
         FROM users u LEFT JOIN institute_settings s ON u.institute_name = s.institute_name"
    ).unwrap();
    let user_iter = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?, user_id: row.get(1)?, full_name: row.get(2)?, role: row.get(3)?,
            institute_name: row.get(4).unwrap_or_default(), hostel_block: row.get(5).unwrap_or_default(),
            wing: row.get(6).unwrap_or_default(), room: row.get(7).unwrap_or_default(),
            mess_assigned: row.get(8).unwrap_or_default(), phone: row.get(9).unwrap_or_default(),
            parent_phone: row.get(10).unwrap_or_default(), password_hash: row.get(11)?,
            photo_locked: row.get(12).unwrap_or(false), profile_pic_url: row.get(13).unwrap_or_default(),
            is_exempt: row.get(14).unwrap_or(false), institute_otp_enabled: row.get::<_, i32>(15).unwrap_or(1) != 0,
        })
    }).unwrap();
    Json(user_iter.map(|u| u.unwrap()).collect())
}

async fn add_user_handler(State(state): State<Arc<AppState>>, Json(payload): Json<NewUserRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    let result = db.execute("INSERT INTO users (user_id, full_name, role, institute_name, hostel_block, wing, room, mess_assigned, phone, parent_phone, password_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)", params![payload.user_id, payload.full_name, payload.role, payload.institute_name, payload.hostel_block, payload.wing, payload.room, payload.mess_assigned, payload.phone, payload.parent_phone, payload.phone]);
    match result { Ok(_) => Ok(Json(serde_json::json!({"success": true, "message": format!("Successfully registered {}", payload.user_id)}))), Err(_) => Ok(Json(serde_json::json!({"success": false, "message": "User ID already exists."}))) }
}

async fn bulk_upload_handler(State(state): State<Arc<AppState>>, Json(payload): Json<Vec<NewUserRequest>>) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut db = state.db.lock().unwrap();
    let tx = db.transaction().unwrap();
    let mut inserted = 0;
    for u in payload {
        let res = tx.execute(
            "INSERT OR IGNORE INTO users (user_id, full_name, role, institute_name, hostel_block, wing, room, mess_assigned, phone, parent_phone, password_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![u.user_id, u.full_name, u.role, u.institute_name, u.hostel_block, u.wing, u.room, u.mess_assigned, u.phone, u.parent_phone, u.phone],
        );
        if let Ok(count) = res { inserted += count; }
    }
    tx.commit().unwrap();
    Ok(Json(serde_json::json!({"success": true, "message": format!("Bulk onboarding successful: {} records processed.", inserted)})))
}

async fn edit_user_handler(State(state): State<Arc<AppState>>, Json(payload): Json<EditUserRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    let result = db.execute("UPDATE users SET role = ?1, institute_name = ?2, hostel_block = ?3, wing = ?4, room = ?5, mess_assigned = ?6 WHERE user_id = ?7", params![payload.new_role, payload.new_institute, payload.new_hostel, payload.new_wing, payload.new_room, payload.new_mess_assigned, payload.target_user_id]);
    match result { Ok(_) => Ok(Json(serde_json::json!({"success": true, "message": format!("Updated details for {}", payload.target_user_id)}))), Err(_) => Ok(Json(serde_json::json!({"success": false, "message": "Failed to update user."}))) }
}

async fn toggle_exemption_handler(State(state): State<Arc<AppState>>, Json(payload): Json<ExemptionRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    let val = if payload.is_exempt { 1 } else { 0 };
    db.execute("UPDATE users SET is_exempt = ?1 WHERE user_id = ?2", params![val, payload.user_id]).unwrap();
    Ok(Json(serde_json::json!({"success": true})))
}

async fn toggle_institute_otp_handler(State(state): State<Arc<AppState>>, Json(payload): Json<InstituteOtpToggle>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    let val = if payload.otp_enabled { 1 } else { 0 };
    db.execute("INSERT INTO institute_settings (institute_name, otp_enabled) VALUES (?1, ?2) ON CONFLICT(institute_name) DO UPDATE SET otp_enabled=excluded.otp_enabled", params![payload.institute_name, val]).unwrap();
    let status_str = if payload.otp_enabled { "ENABLED" } else { "DISABLED" };
    Ok(Json(serde_json::json!({"success": true, "message": format!("OTP requirement {} for {}.", status_str, payload.institute_name)})))
}

async fn delete_user_handler(State(state): State<Arc<AppState>>, Path(target_user_id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    db.execute("DELETE FROM users WHERE user_id = ?1", params![target_user_id]).unwrap();
    Ok(Json(serde_json::json!({"success": true, "message": format!("Deleted user {}", target_user_id)})))
}

async fn update_profile_pic_handler(State(state): State<Arc<AppState>>, Json(payload): Json<PicPayload>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    let locked: bool = db.query_row("SELECT photo_locked FROM users WHERE user_id = ?1", params![payload.user_id], |r| r.get(0)).unwrap_or(false);
    if locked { return Ok(Json(serde_json::json!({"success": false, "message": "Profile picture is already locked!"}))); }
    db.execute("UPDATE users SET profile_pic_url = ?1, photo_locked = 1 WHERE user_id = ?2", params![payload.profile_pic_url, payload.user_id]).unwrap();
    Ok(Json(serde_json::json!({"success": true, "message": "Profile picture permanently locked!"})))
}

async fn update_timer_handler(State(state): State<Arc<AppState>>, Json(payload): Json<Timer>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap();
    db.execute("INSERT INTO timers (hostel_block, timer_type, start_time, end_time) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(hostel_block, timer_type) DO UPDATE SET start_time=excluded.start_time, end_time=excluded.end_time", params![payload.hostel_block, payload.timer_type, payload.start_time, payload.end_time]).unwrap();
    Json(serde_json::json!({"success": true, "message": format!("{} timing updated successfully.", payload.timer_type)}))
}

async fn get_timers_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<Vec<Timer>> {
    let db = state.db.lock().unwrap();
    let hostel_target = query.q.unwrap_or_default();
    let mut stmt = db.prepare("SELECT hostel_block, timer_type, start_time, end_time FROM timers WHERE hostel_block = ?1").unwrap();
    let iter = stmt.query_map(params![hostel_target], |row| Ok(Timer { hostel_block: row.get(0)?, timer_type: row.get(1)?, start_time: row.get(2)?, end_time: row.get(3)? })).unwrap();
    Json(iter.map(|t| t.unwrap()).collect())
}

async fn smart_search_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<Vec<serde_json::Value>> {
    let db = state.db.lock().unwrap();
    let search_term = format!("%{}%", query.q.unwrap_or_default().replace(" ", "").to_lowercase());
    let mess_filter = query.mess_filter.unwrap_or_default();
    let wing_filter = query.wing_filter.unwrap_or_default();
    
    let mut stmt = db.prepare(
        "SELECT user_id, full_name, hostel_block, wing, room, profile_pic_url, mess_assigned,
         (SELECT GROUP_CONCAT(meal_type) FROM attendance WHERE student_id = users.user_id AND date_logged = CURRENT_DATE) as meals_today
         FROM users 
         WHERE role = 'Student' 
         AND (LOWER(user_id) LIKE ?1 OR LOWER(full_name) LIKE ?1 OR LOWER(room) LIKE ?1 OR LOWER(wing) LIKE ?1 OR LOWER(REPLACE(wing || room, ' ', '')) LIKE ?1)
         AND (?2 = '' OR mess_assigned = ?2) 
         AND (?3 = '' OR wing = ?3)"
    ).unwrap();

    let iter = stmt.query_map(params![search_term, mess_filter, wing_filter], |row| { 
        Ok(serde_json::json!({ 
            "user_id": row.get::<_, String>(0)?, "full_name": row.get::<_, String>(1)?, 
            "hostel_block": row.get::<_, String>(2)?, "wing": row.get::<_, String>(3)?, "room": row.get::<_, String>(4)?, 
            "profile_pic_url": row.get::<_, String>(5).unwrap_or_default(), "mess_assigned": row.get::<_, String>(6).unwrap_or_default(),
            "meals_today": row.get::<_, Option<String>>(7).unwrap_or_default()
        })) 
    }).unwrap();
    Json(iter.map(|u| u.unwrap()).collect())
}

async fn mark_present_handler(State(state): State<Arc<AppState>>, Json(payload): Json<MarkAttendanceRequest>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap();
    db.execute("INSERT INTO attendance (student_id, meal_type) VALUES (?1, ?2)", params![payload.student_id, payload.meal_type]).unwrap();
    Json(serde_json::json!({"success": true, "message": format!("{} marked present.", payload.student_id)}))
}

async fn trigger_sweep_handler(State(state): State<Arc<AppState>>, Json(payload): Json<MarkAttendanceRequest>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT user_id, parent_phone FROM users WHERE role = 'Student' AND is_exempt = 0 AND user_id NOT IN (SELECT student_id FROM attendance WHERE meal_type = ?1 AND date_logged = CURRENT_DATE)").unwrap();
    let absentees_iter = stmt.query_map(params![payload.meal_type], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap();
    let mut count = 0;
    
    println!("========================================");
    println!("🟢 [WHATSAPP BUSINESS AUTOMATED SWEEP]");
    for absentee in absentees_iter {
        let (id, parent_phone) = absentee.unwrap(); count += 1;
        println!("Message to +91 {}: *Alert*: Student {} did not record attendance for {}.", parent_phone, id, payload.meal_type);
    }
    println!("========================================");

    Json(serde_json::json!({"success": true, "message": format!("Sweep complete. {} missing students flagged. Automated WhatsApp Alert dispatched.", count)}))
}

async fn get_complaints_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<Vec<Complaint>> {
    let db = state.db.lock().unwrap();
    let filter = query.hostel_filter.unwrap_or_default();
    let role = query.role_filter.unwrap_or_default();

    let mut sql = "SELECT id, student_id, student_name, hostel_block, wing, room, category, description, status FROM complaints WHERE (?1 = '' OR hostel_block = ?1)".to_string();
    
    if role.starts_with("MaintenanceStaff_") {
        sql.push_str(" AND status = 'Active'");
    }
    
    let mut stmt = db.prepare(&sql).unwrap();
    let iter = stmt.query_map(params![filter], |row| Ok(Complaint { id: row.get(0)?, student_id: row.get(1)?, student_name: row.get(2)?, hostel_block: row.get(3)?, wing: row.get(4)?, room: row.get(5)?, category: row.get(6)?, description: row.get(7)?, status: row.get(8)? })).unwrap();
    
    let mut comps: Vec<Complaint> = iter.map(|c| c.unwrap()).collect();
    if role.starts_with("MaintenanceStaff_") {
        let specialized_cat = role.replace("MaintenanceStaff_", "");
        comps.retain(|c| c.category == specialized_cat);
    }
    
    Json(comps)
}

async fn raise_complaint_handler(State(state): State<Arc<AppState>>, Json(payload): Json<Complaint>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    db.execute("INSERT INTO complaints (student_id, student_name, hostel_block, wing, room, category, description, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'Active')", 
    params![payload.student_id, payload.student_name, payload.hostel_block, payload.wing, payload.room, payload.category, payload.description]).unwrap();
    Ok(Json(serde_json::json!({"success": true})))
}

async fn resolve_complaint_handler(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    db.execute("UPDATE complaints SET status = 'Resolved' WHERE id = ?1", params![id]).unwrap();
    Ok(Json(serde_json::json!({"success": true})))
}

async fn get_fines_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<Vec<Fine>> {
    let db = state.db.lock().unwrap();
    let hostel = query.hostel_filter.unwrap_or_default();
    let student = query.q.unwrap_or_default();
    let mut stmt = db.prepare("SELECT id, student_id, student_name, hostel_block, amount, reason, status, date_issued FROM fines WHERE (?1 = '' OR hostel_block = ?1) AND (?2 = '' OR LOWER(student_id) = LOWER(?2))").unwrap();
    let iter = stmt.query_map(params![hostel, student], |row| Ok(Fine { id: row.get(0)?, student_id: row.get(1)?, student_name: row.get(2)?, hostel_block: row.get(3)?, amount: row.get(4)?, reason: row.get(5)?, status: row.get(6)?, date_issued: row.get(7)? })).unwrap();
    Json(iter.map(|f| f.unwrap()).collect())
}

async fn issue_fine_handler(State(state): State<Arc<AppState>>, Json(payload): Json<Fine>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    let student_name: String = db.query_row("SELECT full_name FROM users WHERE user_id = ?1", params![payload.student_id], |r| r.get(0)).unwrap_or_else(|_| "Student".to_string());
    db.execute("INSERT INTO fines (student_id, student_name, hostel_block, amount, reason, status) VALUES (?1, ?2, ?3, ?4, ?5, 'Unpaid')", params![payload.student_id, student_name, payload.hostel_block, payload.amount, payload.reason]).unwrap();
    Ok(Json(serde_json::json!({"success": true, "message": "Disciplinary fine issued."})))
}

async fn pay_fine_handler(State(state): State<Arc<AppState>>, Path(id): Path<u32>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    db.execute("UPDATE fines SET status = 'Paid' WHERE id = ?1", params![id]).unwrap();
    Ok(Json(serde_json::json!({"success": true, "message": "Fine cleared."})))
}

async fn get_leaves_handler(State(state): State<Arc<AppState>>, Query(query): Query<SearchQuery>) -> Json<Vec<LeaveRequest>> {
    let db = state.db.lock().unwrap();
    let hostel = query.hostel_filter.unwrap_or_default();
    let student = query.q.unwrap_or_default();
    let mut stmt = db.prepare("SELECT id, student_id, student_name, hostel_block, wing, room, start_date, end_date, days_count, reason, status, pass_code FROM leave_requests WHERE (?1 = '' OR hostel_block = ?1) AND (?2 = '' OR LOWER(student_id) = LOWER(?2))").unwrap();
    let iter = stmt.query_map(params![hostel, student], |row| Ok(LeaveRequest { id: row.get(0)?, student_id: row.get(1)?, student_name: row.get(2)?, hostel_block: row.get(3)?, wing: row.get(4)?, room: row.get(5)?, start_date: row.get(6)?, end_date: row.get(7)?, days_count: row.get(8)?, reason: row.get(9)?, status: row.get(10)?, pass_code: row.get(11)? })).unwrap();
    Json(iter.map(|l| l.unwrap()).collect())
}

async fn apply_leave_handler(State(state): State<Arc<AppState>>, Json(payload): Json<LeaveRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    let time_nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    let generated_pass = format!("PASS-{:06}", time_nanos % 1000000);
    db.execute("INSERT INTO leave_requests (student_id, student_name, hostel_block, wing, room, start_date, end_date, days_count, reason, status, pass_code) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'Pending', ?10)", params![payload.student_id, payload.student_name, payload.hostel_block, payload.wing, payload.room, payload.start_date, payload.end_date, payload.days_count, payload.reason, generated_pass]).unwrap();
    Ok(Json(serde_json::json!({"success": true, "message": "Leave application dispatched to Warden for authorization."})))
}

async fn approve_leave_handler(State(state): State<Arc<AppState>>, Json(payload): Json<LeaveApprovalRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    db.execute("UPDATE leave_requests SET status = ?1 WHERE id = ?2", params![payload.status, payload.leave_id]).unwrap();
    let is_exempt = if payload.status == "Approved" { 1 } else { 0 };
    db.execute("UPDATE users SET is_exempt = ?1 WHERE user_id = ?2", params![is_exempt, payload.student_id]).unwrap();
    Ok(Json(serde_json::json!({"success": true, "message": format!("Leave status updated to: {}", payload.status)})))
}

async fn verify_gate_pass_handler(State(state): State<Arc<AppState>>, Json(payload): Json<GatePassVerifyRequest>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT student_id, student_name, wing, room, start_date, end_date, status FROM leave_requests WHERE UPPER(pass_code) = UPPER(?1)").unwrap();
    let result = stmt.query_row(params![payload.pass_code.trim()], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?, r.get::<_, String>(6)?)));
    match result {
        Ok((sid, sname, wing, room, sdate, edate, status)) => {
            if status == "Approved" { Ok(Json(serde_json::json!({ "valid": true, "student_id": sid, "student_name": sname, "location": format!("Wing {}, Room {}", wing, room), "duration": format!("{} to {}", sdate, edate) }))) } 
            else { Ok(Json(serde_json::json!({"valid": false, "message": format!("Out-pass is not approved (Current status: {})", status)}))) }
        },
        _ => Ok(Json(serde_json::json!({"valid": false, "message": "Invalid or expired digital pass code."}))),
    }
}

async fn get_notices_handler(State(state): State<Arc<AppState>>) -> Json<Vec<Notice>> {
    let db = state.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT id, author_name, title, content, category, date_posted FROM notices ORDER BY id DESC").unwrap();
    let iter = stmt.query_map([], |row| Ok(Notice { id: row.get(0)?, author_name: row.get(1)?, title: row.get(2)?, content: row.get(3)?, category: row.get(4)?, date_posted: row.get(5)? })).unwrap();
    Json(iter.map(|n| n.unwrap()).collect())
}

async fn post_notice_handler(State(state): State<Arc<AppState>>, Json(payload): Json<Notice>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap();
    db.execute("INSERT INTO notices (author_name, title, content, category) VALUES (?1, ?2, ?3, ?4)", params![payload.author_name, payload.title, payload.content, payload.category]).unwrap();
    Json(serde_json::json!({"success": true, "message": "Notice published to all dashboards."}))
}

async fn trigger_sos_handler(State(state): State<Arc<AppState>>, Json(payload): Json<SOSRequest>) -> Json<serde_json::Value> {
    let db = state.db.lock().unwrap();
    db.execute("INSERT INTO sos_alerts (student_id, wing, room) VALUES (?1, ?2, ?3)", params![payload.student_id, payload.wing, payload.room]).unwrap();
    println!("🚨🚨🚨 CRITICAL SOS ACTIVATED 🚨🚨🚨");
    println!("Location: {}, Wing {}, Room {}", payload.hostel_block, payload.wing, payload.room);
    println!("Student: {} ({})", payload.student_name, payload.student_id);
    println!("Dispatching Security Immediately.");
    Json(serde_json::json!({"success": true}))
}

#[tokio::main]
async fn main() {
    let conn = init_db();
    let shared_state = Arc::new(AppState { db: Mutex::new(conn), active_otps: Mutex::new(HashMap::new()) });
    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/api/auth/login", post(login_handler))
        .route("/api/users", get(get_users_handler))
        .route("/api/admin/add-user", post(add_user_handler))
        .route("/api/admin/bulk-upload", post(bulk_upload_handler))
        .route("/api/admin/edit-user", post(edit_user_handler))
        .route("/api/admin/delete-user/:id", delete(delete_user_handler))
        .route("/api/admin/toggle-exemption", post(toggle_exemption_handler))
        .route("/api/admin/toggle-institute-otp", post(toggle_institute_otp_handler))
        .route("/api/user/update-pic", post(update_profile_pic_handler))
        .route("/api/timers", get(get_timers_handler).post(update_timer_handler))
        .route("/api/warden/stats", get(get_warden_stats_handler))
        .route("/api/ai/insights", get(get_ai_insights_handler))
        .route("/api/mess/search", get(smart_search_handler))
        .route("/api/mess/mark", post(mark_present_handler))
        .route("/api/warden/sweep", post(trigger_sweep_handler))
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
        .with_state(shared_state)
        .layer(cors);

  let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
let addr = format!("0.0.0.0:{}", port);
let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
println!("🚀 HMS Enterprise Backend Online at http://{}", addr);
axum::serve(listener, app).await.unwrap();
}
// Add this at the top with your other imports
use tower_http::cors::CorsLayer;

// ... down where you create your router:
let app = Router::new()
    // your routes go here, e.g., .route("/api/auth/login", post(login_handler))
    .layer(CorsLayer::permissive()); // This line allows phones and frontends to connect!