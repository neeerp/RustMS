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

#include "../Configuration.h"
#include "../Constants.h"
#include "../Util/NxFiles.h"

namespace ms
{
	namespace
	{
		constexpr int16_t STEP_HEIGHT = 600;

		// The original client's camera sits slightly right of and above
		// each stop, measured against period footage of every step.
		constexpr double CAMERA_X = 28;
		constexpr double CAMERA_Y = -8;

		// The share of the remaining distance the camera covers each update.
		constexpr double SCROLL_RATE = 0.06;

		// Where the step sign rests, and the left edge it slides in from.
		constexpr int16_t SIGN_Y = 31;
		constexpr double SIGN_START_X = -150;

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

	void LoginScene::load()
	{
		nl::node map = NxFiles::ui_classic["MapLogin.img"];

		backgrounds = MapBackgrounds(map["back"]);
		tilesobjs = MapTilesObjs(map);
		frame = NxFiles::ui_classic["Login.img"]["Common"]["frame"];
		version = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::BLACK,
			"Ver. " + Configuration::get().get_version());

		loaded = true;
		visible = false;
	}

	void LoginScene::unload()
	{
		backgrounds = {};
		tilesobjs = {};
		sign = {};
		sign_id = 0;
		loaded = false;
		visible = false;
	}

	void LoginScene::move_to(Step step)
	{
		if (!loaded)
			return;

		target_y = CAMERA_Y - STEP_HEIGHT * step;

		if (!visible)
		{
			y = target_y;
			last_y = target_y;
			visible = true;
		}

		uint8_t id = sign_for(step);

		if (id != sign_id)
		{
			sign_id = id;
			sign = id ? Texture(NxFiles::ui_classic["Login.img"]["Common"]["step"][std::to_string(id)]) : Texture();
			sign_x = SIGN_START_X;
			last_sign_x = SIGN_START_X;
		}
	}

	void LoginScene::draw(float alpha) const
	{
		if (!visible)
			return;

		double width = Constants::Constants::get().get_viewwidth();
		double height = Constants::Constants::get().get_viewheight();

		// The camera keeps the same x; only its height changes.
		double centre_y = last_y + (y - last_y) * alpha;
		double view_x = width / 2 - CAMERA_X;
		double view_y = height / 2 - centre_y;
		Point<int16_t> viewpos(static_cast<int16_t>(view_x), static_cast<int16_t>(std::round(view_y)));

		backgrounds.drawbackgrounds(view_x, view_y, alpha);

		for (auto id : Layer::IDs)
			tilesobjs.draw(id, viewpos, alpha);

		backgrounds.drawforegrounds(view_x, view_y, alpha);

		frame.draw(Point<int16_t>(400, 300));
		version.draw(Point<int16_t>(595, 5));

		double x = last_sign_x + (sign_x - last_sign_x) * alpha;
		sign.draw(Point<int16_t>(static_cast<int16_t>(std::round(x)), SIGN_Y));
	}

	void LoginScene::update()
	{
		if (!visible)
			return;

		backgrounds.update();
		tilesobjs.update();

		last_y = y;

		double delta = target_y - y;

		if (std::abs(delta) < 1.0)
			y = target_y;
		else
			y += delta * SCROLL_RATE;

		// The sign eases in the same way the camera does.
		last_sign_x = sign_x;

		if (std::abs(sign_x) < 1.0)
			sign_x = 0.0;
		else
			sign_x -= sign_x * SCROLL_RATE;
	}
}
