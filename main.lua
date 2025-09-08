-- Global variable to track proxy process
local proxy_process = nil
local proxy_port = "12081"

-- Load website configuration from config file
local function load_website_config()
    local script_dir = mp.get_script_directory()
    local config_path = script_dir .. "/config.toml"

    -- Default configuration
    local config = {
        youtube = true,
        youtube_alternatives = true,
        vimeo = false,
        dailymotion = false,
        twitch = false,
        custom_domains = {}
    }

    -- Try to parse config file (simplified TOML parsing)
    local f = io.open(config_path, "r")
    if f then
        local success, content = pcall(function() return f:read("*all") end)
        pcall(function() f:close() end)

        if success and content then
            -- Find [websites] section start
            local websites_start = content:find("%[websites%]")
            if websites_start then
                -- Get text after [websites] section
                local after_websites = content:sub(websites_start)

                -- Parse boolean values with error handling
                for key, value in after_websites:gmatch("(%w+)%s*=%s*(%w+)") do
                    if key == "youtube" or key == "youtube_alternatives" or
                        key == "vimeo" or key == "dailymotion" or key == "twitch" then
                        config[key] = (value:lower() == "true")
                    end
                end

                -- Parse custom_domains with error handling
                local domains_line = after_websites:match("custom_domains%s*=%s*%[([^%]]+)%]")
                if domains_line then
                    config.custom_domains = {}
                    for domain in domains_line:gmatch('"([^"]+)"') do
                        -- Basic domain validation
                        if domain and #domain > 0 and not domain:match("^%s*$") then
                            table.insert(config.custom_domains, domain)
                        end
                    end
                end
            else
                -- Debug: Log if websites section not found
                -- print("DEBUG: [websites] section not found in config")
            end
        end
    else
        -- Debug: Log if config file not found
        -- print("DEBUG: Config file not found: " .. config_path)
    end

    return config
end

-- Enhanced URL validation with configurable website support
local function is_supported_url(url, config)
    if not url or type(url) ~= "string" then
        return false
    end

    -- YouTube patterns
    if config.youtube then
        local youtube_patterns = {
            "^https://[%w%-%.]*youtube%.com/",
            "^https://[%w%-%.]*youtu%.be/"
        }
        for _, pattern in ipairs(youtube_patterns) do
            if url:match(pattern) then
                return true
            end
        end
    end

    -- YouTube alternatives
    if config.youtube_alternatives then
        local alt_patterns = {
            "^https://[%w%-%.]*yewtu%.be/",
            "^https://[%w%-%.]*invidio%.us/",
            "^https://[%w%-%.]*piped%.video/"
        }
        for _, pattern in ipairs(alt_patterns) do
            if url:match(pattern) then
                return true
            end
        end
    end

    -- Vimeo
    if config.vimeo and url:match("^https://[%w%-%.]*vimeo%.com/") then
        return true
    end

    -- Dailymotion
    if config.dailymotion and url:match("^https://[%w%-%.]*dailymotion%.com/") then
        return true
    end

    -- Twitch
    if config.twitch and url:match("^https://[%w%-%.]*twitch%.tv/") then
        return true
    end

    -- Custom domains
    for _, domain in ipairs(config.custom_domains) do
        -- Simple string.find is more reliable than complex pattern matching
        if url:find(domain, 1, true) then -- plain text search
            return true
        end
    end

    return false
end

-- Better URL validation for YouTube/Yewtu.be (kept for backward compatibility)
local function is_youtube_url(url)
    local config = {
        youtube = true,
        youtube_alternatives = true,
        vimeo = false,
        dailymotion = false,
        twitch = false,
        custom_domains = {}
    }
    return is_supported_url(url, config)
end

-- Cleanup function to stop proxy and restore settings
local function cleanup_proxy()
    if proxy_process then
        mp.msg.info("Stopping HTTP proxy process")
        -- Note: mpv doesn't provide direct process killing,
        -- but the process should terminate when mpv exits
        proxy_process = nil
    end

    -- Restore original settings
    mp.set_property("tls-verify", "yes")
    mp.set_property("http-proxy", "")
    mp.msg.info("Restored original TLS and proxy settings")
end

local function init()
    local opts = mp.get_property_native("options/script-opts")
    if opts and opts["http-ytproxy"] == "no" then
        mp.msg.info("HTTP YouTube proxy disabled by script-opts")
        return
    end

    local url = mp.get_property("stream-open-filename")

    -- Load website configuration
    local website_config = load_website_config()

    -- Check if URL is supported based on configuration
    if not is_supported_url(url, website_config) then
        return
    end

    mp.msg.info("Supported video URL detected: " .. url)

    local proxy = mp.get_property("http-proxy")
    local ytdl_raw_options = mp.get_property("ytdl-raw-options")

    -- Check if another proxy is already configured
    local our_proxy = "http://127.0.0.1:" .. proxy_port
    if (proxy and proxy ~= "" and proxy ~= our_proxy) or
        (ytdl_raw_options and ytdl_raw_options:match("proxy=([^ ]+)")) then
        mp.msg.warn("Another proxy is already configured, skipping YouTube proxy")
        return
    end

    -- Skip if our proxy is already running
    if proxy == our_proxy then
        mp.msg.info("YouTube proxy already active")
        return
    end

    -- Check if binary exists
    local script_dir = mp.get_script_directory()
    local binary_path = script_dir .. "/http-ytproxy"
    local cert_path = script_dir .. "/cert.pem"
    local key_path = script_dir .. "/key.pem"

    -- Validate required files exist (improved error handling)
    local function file_exists(path)
        local f = io.open(path, "r")
        if f then
            local success = pcall(function() f:close() end)
            return true
        end
        return false
    end

    if not file_exists(binary_path) then
        mp.msg.error("HTTP proxy binary not found: " .. binary_path)
        return
    end

    if not file_exists(cert_path) or not file_exists(key_path) then
        mp.msg.error("Certificate files not found in: " .. script_dir)
        return
    end

    mp.msg.info("Starting HTTP YouTube proxy on port " .. proxy_port)

    -- launch mitm proxy
    local args = {
        binary_path,
        "-c", cert_path,
        "-k", key_path,
        "--config", script_dir .. "/config.toml",
        "-p", proxy_port
    }

    proxy_process = mp.command_native_async({
        name = "subprocess",
        capture_stdout = false,
        playback_only = false,
        args = args,
    })

    mp.set_property("http-proxy", "http://127.0.0.1:" .. proxy_port)
    mp.set_property("tls-verify", "no")

    mp.msg.info("HTTP YouTube proxy activated")

    -- Note: TLS verification is disabled for the proxy to work
    -- This is safe only when using localhost proxy with self-signed certs
    -- Alternative: Enable TLS verification with custom CA
    -- mp.set_property("tls-verify", "yes")
    -- mp.set_property("tls-ca-file", cert_path)
end

-- Register events
mp.register_event("start-file", init)
mp.register_event("shutdown", cleanup_proxy)
mp.register_event("end-file", cleanup_proxy)
