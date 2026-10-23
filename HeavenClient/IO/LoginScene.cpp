//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "LoginScene.h"

#include "../Audio/Audio.h"
#include "../Configuration.h"
#include "../Constants.h"
#include "../Util/NxFiles.h"

#include <algorithm>
#include <cmath>

namespace ms
{
	namespace
	{
		constexpr int16_t STEP_HEIGHT = 600;

		// The original client's camera sits slightly right of and above
		// each stop: FUN_00569760 puts step n at -600n - 8.
		constexpr double CAMERA_X = 28;
		constexpr double CAMERA_Y = -8;

		// CLogin::ChangeStep flies for 300 ms per step plus 500 ms, leaving
		// at 3.5 times the distance per flight and arriving at rest.
		constexpr double FLY_TIME_PER_STEP = 300;
		constexpr double FLY_TIME = 500;
		constexpr double FLY_LAUNCH = 3.5;

		// Step signs rest at the top left and slide in from x = -149; the
		// world box rests under them and slides in a little slower. Both
		// fade out in place, and get their shadow once they are in place.
		constexpr int16_t SIGN_START_X = -149;
		constexpr int16_t SIGN_Y = 33;
		constexpr double SIGN_SLIDE_TIME = 300;
		constexpr int16_t WORLD_BOX_Y = 96;
		constexpr double WORLD_BOX_SLIDE_TIME = 400;
		constexpr double FADE_TIME = 300;
		constexpr double SHADOW_TIME = 1000;

		// The world and channel names, centred on the world box.
		const Point<int16_t> WORLD_NAME_POS = Point<int16_t>(100, 8);
		const Point<int16_t> CHANNEL_NAME_POS = Point<int16_t>(100, 23);

		double stop_for(LoginScene::Step step)
		{
			return CAMERA_Y - STEP_HEIGHT * step;
		}

		// Login.img/Common/step art: 1 world, 2 character, 3 creation.
		uint8_t sign_for(LoginScene::Step step)
		{
			switch (step)
			{
			case LoginScene::Step::TITLE:
				return 0;
			case LoginScene::Step::WORLD_SELECT:
				return 1;
			case LoginScene::Step::CHAR_SELECT:
				return 2;
			default:
				return 3;
			}
		}
	}

	int16_t LoginScene::Panel::x(double time) const
	{
		// A closed panel stays where it was.
		double elapsed = (closed < 0.0 ? time : std::min(time, closed)) - opened;

		if (elapsed >= slide_time)
			return 0;

		if (elapsed <= 0.0)
			return SIGN_START_X;

		return SIGN_START_X + static_cast<int16_t>(-SIGN_START_X * elapsed / slide_time);
	}

	float LoginScene::Panel::opacity(double time) const
	{
		if (closed < 0.0)
			return 1.0f;

		return static_cast<float>(std::clamp(1.0 - (time - closed) / FADE_TIME, 0.0, 1.0));
	}

	float LoginScene::Panel::shadow_opacity(double time) const
	{
		double landed = opened + slide_time;

		if (closed >= 0.0 && closed < landed)
			return 0.0f;

		return static_cast<float>(std::clamp((time - landed) / SHADOW_TIME, 0.0, 1.0)) * opacity(time);
	}

	bool LoginScene::Panel::gone(double time) const
	{
		return closed >= 0.0 && time - closed >= FADE_TIME;
	}

	void LoginScene::load()
	{
		nl::node map = NxFiles::ui_classic["MapLogin.img"];
		nl::node common = NxFiles::ui_classic["Login.img"]["Common"];

		backgrounds = MapBackgrounds(map["back"]);
		// The login map has no footholds, and none of its objs drift.
		tilesobjs = MapTilesObjs(map, {});
		frame = common["frame"];
		shadow = common["shadow"]["0"];
		version = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::BLACK,
			"Ver. " + Configuration::get().get_version());

		now = 0.0;
		flying = false;
		loaded = true;
		visible = false;
	}

	void LoginScene::unload()
	{
		backgrounds = {};
		tilesobjs = {};
		sign.reset();
		sign_id = 0;
		world_box.reset();
		loaded = false;
		visible = false;
	}

	void LoginScene::move_to(Step next)
	{
		if (!loaded)
			return;

		if (!visible)
		{
			visible = true;
			step = next;
			from_y = stop_for(next);
			to_y = from_y;
			flying = false;
			arrive();

			return;
		}

		if (next == step)
			return;

		// Every change of step scrolls up (v83 FUN_005f53c0, 5f5938).
		Sound(Sound::Name::SCROLLUP).play();

		// The flight leaves from the old step's stop, wherever the camera is.
		double steps = std::abs(static_cast<int>(next) - static_cast<int>(step));
		from_y = stop_for(step);
		to_y = stop_for(next);
		fly_start = now;
		fly_end = now + FLY_TIME_PER_STEP * steps + FLY_TIME;
		flying = true;
		step = next;

		// Race select and creation share their sign, so it stays up.
		if (sign && sign->closed < 0.0 && sign_for(next) != sign_id)
		{
			sign->closed = now;
			sign_id = 0;
		}

		if (world_box && world_box->closed < 0.0 && next <= Step::WORLD_SELECT)
			world_box->closed = now;
	}

	bool LoginScene::moving() const
	{
		return flying;
	}

	void LoginScene::set_world(const std::string& world, const std::string& channel)
	{
		world_name = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::GALLERY, world);
		channel_name = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::GALLERY, channel);
	}

	void LoginScene::arrive()
	{
		uint8_t id = sign_for(step);

		if (id && id != sign_id)
			open_sign(id);

		if (step >= Step::CHAR_SELECT && (!world_box || world_box->closed >= 0.0))
		{
			world_box = Panel();
			world_box->art = NxFiles::ui_classic["Login.img"]["Common"]["selectWorld"];
			world_box->y = WORLD_BOX_Y;
			world_box->slide_time = WORLD_BOX_SLIDE_TIME;
			world_box->opened = now;
		}
	}

	void LoginScene::open_sign(uint8_t id)
	{
		sign = Panel();
		sign->art = NxFiles::ui_classic["Login.img"]["Common"]["step"][std::to_string(id)];
		sign->y = SIGN_Y;
		sign->slide_time = SIGN_SLIDE_TIME;
		sign->opened = now;
		sign_id = id;
	}

	double LoginScene::draw_time(float alpha) const
	{
		// Everything here moves on the clock, so draw it between updates.
		return now - (1.0 - alpha) * Constants::TIMESTEP;
	}

	void LoginScene::draw_panel(const Panel& panel, double time) const
	{
		Point<int16_t> position(panel.x(time), panel.y);

		if (float opacity = panel.shadow_opacity(time); opacity > 0.0f)
			shadow.draw(DrawArgument(position, opacity));

		panel.art.draw(DrawArgument(position, panel.opacity(time)));
	}

	void LoginScene::draw(float alpha) const
	{
		if (!visible)
			return;

		double time = draw_time(alpha);

		double width = Constants::Constants::get().get_viewwidth();
		double height = Constants::Constants::get().get_viewheight();

		// The camera keeps the same x; its height follows a Hermite curve
		// between the stops, cut to whole pixels.
		double centre_y = to_y;

		if (flying && time < fly_end)
		{
			double s = std::clamp((time - fly_start) / (fly_end - fly_start), 0.0, 1.0);
			double launch = FLY_LAUNCH * (to_y - from_y);
			double s2 = s * s;
			double s3 = s2 * s;
			centre_y = std::trunc(from_y * (2 * s3 - 3 * s2 + 1) + launch * (s3 - 2 * s2 + s)
				+ to_y * (3 * s2 - 2 * s3));
		}

		double view_x = width / 2 - CAMERA_X;
		double view_y = height / 2 - centre_y;
		Point<int16_t> viewpos(static_cast<int16_t>(view_x), static_cast<int16_t>(view_y));

		backgrounds.drawbackgrounds(view_x, view_y, alpha);

		for (auto id : Layer::IDs)
			tilesobjs.draw(id, viewpos, alpha);

		backgrounds.drawforegrounds(view_x, view_y, alpha);

		frame.draw(Point<int16_t>(400, 300));
		version.draw(Point<int16_t>(595, 5));

		if (sign)
			draw_panel(*sign, time);
	}

	void LoginScene::draw_world_box(float alpha) const
	{
		if (!visible || !world_box)
			return;

		// The names are written on the box, so they slide and fade with it.
		double time = draw_time(alpha);
		draw_panel(*world_box, time);

		Point<int16_t> position(world_box->x(time), world_box->y);
		float opacity = world_box->opacity(time);
		world_name.draw(DrawArgument(position + WORLD_NAME_POS, opacity));
		channel_name.draw(DrawArgument(position + CHANNEL_NAME_POS, opacity));
	}

	void LoginScene::update()
	{
		if (!visible)
			return;

		backgrounds.update();
		tilesobjs.update();

		now += Constants::TIMESTEP;

		if (flying && now > fly_end)
		{
			flying = false;
			arrive();
		}

		if (sign && sign->gone(now))
			sign.reset();

		if (world_box && world_box->gone(now))
			world_box.reset();
	}
}
