module main

import vweb
import net.http
import json

struct OnboardingUpdate {
	onboarding_complete bool
}

@['/api/user/onboarding'; post]
pub fn (mut app App) update_onboarding() vweb.Result {
	payload := json.decode(OnboardingUpdate, app.req.data) or {
		return app.server_error_msg('Invalid payload')
	}
	
	println('Updating onboarding status: ${payload}')
	// TODO: Store in persistent state or forward to Tableware
	
	return app.ok('')
}

@['/api/pickups/current'; get]
pub fn (mut app App) get_current_pickup() vweb.Result {
	session_token := app.get_cookie('soliloquy_session') or {
		app.set_status(401, '')
		return app.text('')
	}
	
	_ := app.sessions[session_token] or {
		app.set_status(401, '')
		return app.text('')
	}
	
	// Forward request to Tableware
	pickup_resp := http.get('${app.config.tableware_base_url}/pickups.current') or {
		return app.server_error_msg('Failed to fetch pickup')
	}
	
	app.set_content_type('application/json')
	return app.text(pickup_resp.body)
}
