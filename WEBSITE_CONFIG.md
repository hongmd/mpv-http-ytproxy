# Website Configuration Guide

## Tổng quan

http-ytproxy bây giờ hỗ trợ cấu hình linh hoạt cho các trang web khác nhau thông qua file config.toml. Bạn có thể bật/tắt proxy cho từng loại website cụ thể.

## Cấu hình Websites

Thêm section `[websites]` vào file `config.toml`:

```toml
[websites]
# Bật/tắt proxy cho các loại website cụ thể
youtube = true               # YouTube (youtube.com, youtu.be)
youtube_alternatives = true  # Yewtu.be, Invidious, Piped
vimeo = false               # Vimeo.com
dailymotion = false         # Dailymotion.com
twitch = false              # Twitch.tv
custom_domains = []         # Thêm domain tùy chỉnh: ["example.com", "video.site.com"]
```

## Các tùy chọn có sẵn

### 1. YouTube Chính thức
```toml
youtube = true  # Bật proxy cho youtube.com và youtu.be
```

### 2. YouTube Alternatives
```toml
youtube_alternatives = true  # Bật proxy cho Yewtu.be, Invidious, Piped
```

### 3. Vimeo
```toml
vimeo = true  # Bật proxy cho vimeo.com
```

### 4. Dailymotion
```toml
dailymotion = true  # Bật proxy cho dailymotion.com
```

### 5. Twitch
```toml
twitch = true  # Bật proxy cho twitch.tv
```

### 6. Custom Domains
```toml
custom_domains = ["video.mysite.com", "stream.example.org"]
```

## Ví dụ cấu hình

### Chỉ YouTube
```toml
[websites]
youtube = true
youtube_alternatives = false
vimeo = false
dailymotion = false
twitch = false
custom_domains = []
```

### Tất cả các trang phổ biến
```toml
[websites]
youtube = true
youtube_alternatives = true
vimeo = true
dailymotion = true
twitch = true
custom_domains = []
```

### Chỉ domain tùy chỉnh
```toml
[websites]
youtube = false
youtube_alternatives = false
vimeo = false
dailymotion = false
twitch = false
custom_domains = ["internal-video.company.com", "training.myorg.net"]
```

## Test cấu hình

Sử dụng CLI để test URL:

```bash
# Test với config mặc định
./http-ytproxy --test-url "https://youtube.com/watch?v=abc123"

# Test với config tùy chỉnh
./http-ytproxy --config config.toml --test-url "https://vimeo.com/12345"

# Test custom domain
./http-ytproxy --config config.toml --test-url "https://video.example.com/stream"
```

## Lưu ý quan trọng

1. **Mặc định**: YouTube và YouTube alternatives được bật, các trang khác tắt
2. **Backward Compatibility**: Cấu hình cũ vẫn hoạt động bình thường
3. **mpv Integration**: File `main.lua` sẽ tự động đọc cấu hình từ `config.toml`
4. **Performance**: Proxy chỉ kích hoạt cho các URL được hỗ trợ

## Cập nhật mpv script

Script `main.lua` đã được cập nhật để:
- Tự động đọc file `config.toml`
- Kiểm tra URL theo cấu hình
- Hiển thị thông báo phù hợp

## Troubleshooting

### Proxy không kích hoạt
1. Kiểm tra file `config.toml` có đúng format không
2. Verify URL domain có trong cấu hình không
3. Sử dụng `--test-url` để debug

### Config không load
1. Đảm bảo file `config.toml` ở cùng thư mục với binary
2. Hoặc sử dụng `--config path/to/config.toml`

### Thêm domain mới
```toml
custom_domains = ["newsite.com", "another-video-site.org"]
```
