module main

import vweb
import json
import net.http
import time

struct GoogleTokenResponse {
	access_token string
	token_type   string
	expires_in   int
	id_token     string
}

struct GoogleUserInfo {
	id      string
	email   string
	name    string
	picture string
}

@['/api/auth/google'; get]
pub fn (mut app App) google_auth() vweb.Result {
	auth_url := 'https://accounts.google.com/o/oauth2/v2/auth?' +
		'client_id=${urllib.query_escape(app.config.google_client_id)}&' +
		'redirect_uri=${urllib.query_escape(app.config.google_redirect_uri)}&' +
		'response_type=code&' +
		'scope=${urllib.query_escape('openid email profile')}&' +
		'access_type=offline&' +
		'prompt=consent'
	
	return app.redirect(auth_url)
}

@['/api/auth/google/callback'; get]
pub fn (mut app App) google_callback() vweb.Result {
	code := app.query['code'] or {
		return app.server_error('Missing authorization code')
	}
	
	// Exchange code for token
	token_data := '{"code":"${code}","client_id":"${app.config.google_client_id}","client_secret":"${app.config.google_client_secret}","redirect_uri":"${app.config.google_redirect_uri}","grant_type":"authorization_code"}'
	
	token_resp := http.post_json('https://oauth2.googleapis.com/token', token_data) or {
		eprintln('Token exchange failed: ${err}')
		return app.server_error('Token exchange failed')
	}
	
	token_result := json.decode(GoogleTokenResponse, token_resp.body) or {
		eprintln('Failed to parse token response: ${err}')
		return app.server_error('Invalid token response')
	}
	
	// Get user info
	user_resp := http.fetch(
		url: 'https://www.googleapis.com/oauth2/v2/userinfo'
		method: .get
		header: http.new_header_from_map({
			http.CommonHeader.authorization: 'Bearer ${token_result.access_token}'
		})
	) or {
		eprintln('User info request failed: ${err}')
		return app.server_error('User info failed')
	}
	
	user_info := json.decode(GoogleUserInfo, user_resp.body) or {
		eprintln('Failed to parse user info: ${err}')
		return app.server_error('Invalid user info')
	}
	
	// Generate session token
	session_token := time.now().unix().str() + user_info.id
	
	app.sessions[session_token] = Session{
		user_id: user_info.id
		email: user_info.email
		name: user_info.name
		picture: user_info.picture
	}
	
	println('User authenticated: ${user_info.email}')
	
	// Set cookie and redirect
	app.set_cookie(http.Cookie{
		name: 'soliloquy_session'
		value: session_token
		path: '/'
		http_only: true
		same_site: .lax
		max_age: 604800
	})
	
	return app.redirect('/')
}

@['/api/auth/user'; get]
pub fn (mut app App) get_user() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	session := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	user := User{
		id: session.user_id
		email: session.email
		name: session.name
		picture: session.picture
		onboarding_complete: false
	}
	
	return app.json(user)
}

@['/api/auth/logout'; post]
pub fn (mut app App) logout() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		return app.ok('')
	}
	
	app.sessions.delete(session_token)
	
	app.set_cookie(http.Cookie{
		name: 'soliloquy_session'
		value: ''
		path: '/'
		http_only: true
		same_site: .lax
		max_age: 0
	})
	
	return app.ok('')
}
