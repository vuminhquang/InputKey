# Chrome Web Store listing — Tiếng Việt

## Tên
InputKey — Bộ gõ tiếng Việt

## Tóm tắt
Bộ gõ tiếng Việt mã nguồn mở cho Telex và VNI. Xử lý cục bộ ngay trên thiết bị.

## Danh mục đề xuất
Tools / Công cụ

## Mô tả chi tiết
InputKey giúp bạn gõ tiếng Việt trực tiếp trong các ô nhập liệu trên website bằng Telex hoặc VNI.

Tính năng chính:
- Gõ Telex và VNI với Unicode.
- Hoạt động với input, textarea và contenteditable thông dụng.
- Bật/tắt chế độ tiếng Việt từ popup, Ctrl + Shift hoặc Alt + Z.
- Hỗ trợ Simple Telex, Backspace dựng lại từ đang gõ, Esc trả phím gốc và Auto Restore deterministic.
- Hoàn tác dấu của cả từ: mặc định Ctrl+Space biến token hiện tại về đúng chuỗi phím vật lý, ví dụ \`refer → rể → Ctrl+Space → refer\`.
- Phím tắt hoàn tác cả từ có thể đổi, tắt hoặc đặt lại.
- Không quảng cáo, không analytics, không remote code.
- Nội dung đang gõ chỉ được xử lý trong bộ nhớ trình duyệt, không được lưu hoặc gửi tới máy chủ của nhà phát triển.
- Mã nguồn công khai: https://github.com/vuminhquang/InputKey

Quyền truy cập website chỉ được dùng để phát hiện phím gõ trong ô nhập liệu và chuyển chuỗi Telex/VNI thành tiếng Việt. Extension bỏ qua ô mật khẩu.

Đây là bộ gõ bên trong Chrome/Edge, không phải IME toàn hệ thống. Một số trang được trình duyệt bảo vệ và một số editor đặc biệt có thể không được hỗ trợ.

## Nhà phát triển / liên hệ hỗ trợ
vu.minh.quang@outlook.com

### Hủy dấu bằng phím lặp
Nhấn lặp phím dấu để hủy luôn khả dụng: \`docss → docs\`, \`tesst → test\`. Các từ tiếng Anh như “password”, “coffee” vẫn được giữ nguyên.

### Rust core dùng chung
Extension Chrome/Edge dùng cùng engine Rust deterministic với các bản native của InputKey, được biên dịch thành WebAssembly cục bộ \`inputkey.wasm\`. Toàn bộ xử lý gõ vẫn diễn ra trên thiết bị.
