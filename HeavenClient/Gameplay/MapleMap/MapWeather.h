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
#pragma once

#include "../WeatherRules.h"

#include "../../Audio/Audio.h"
#include "../../Graphics/EffectLayer.h"
#include "../../Graphics/Text.h"

#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// A field's weather: the 512xxxx cash items' particles, bursts and fireworks, their message bar, and the
	// weathers a map's `weather` node blows on its own (CField::BlowWeather FUN_00534aca).
	class MapWeather
	{
	public:
		// The weathers of a map's `weather` node, and the BGM a stopping weather goes back to.
		MapWeather(nl::node src, std::string bgm);
		MapWeather();

		// Start an item's weather with its message, or stop the one blowing with item 0.
		void blow(int32_t item_id, const std::string &msg, bool from_map);

		// BLOW_WEATHER's flag: when set, a stop leaves the message bar up.
		void set_packet_flag(uint8_t flag);

		// Whether a weather item has to wait (FUN_00a0ec3b): while bursts or fireworks go on, or any particles are
		// left. These stay after a stop, fading to nothing, until the next weather or another map.
		bool busy() const;

		// A monster left the field, which drops the map's weathers that wait for none to.
		void mob_left();

		// The BGM a field effect switched to, which a stopping weather plays again instead of the map's.
		void set_bgm_override(std::string path);

		void update();

		// The bursts and fireworks, under the particles.
		void draw_shows(float alpha) const;

		void draw(double viewx, double viewy, float alpha) const;

		// The message bar, above both.
		void draw_bar() const;

		// Silence the weather's sounds as the field goes.
		void stop_sounds();

	private:
		struct Drop
		{
			weather::Particle particle;
			std::optional<Animation> animation;
			int64_t born;
			// When the weather stopped and how it fades from there; -1 while it blows.
			int64_t stopped = -1;
			float stop_alpha = 0.0f;
			int32_t stop_length = 0;
		};

		struct Show
		{
			Animation animation;
			int32_t interval;
			int32_t count;
			int64_t next;
			int64_t end;
			weather::Spread spread;
		};

		struct Fireworks
		{
			Animation animation;
			Sound sound;
			bool sounding = false;
			int64_t next;
			int64_t end;
		};

		struct Timed
		{
			int64_t time;
			int32_t item;
			int32_t option;
			std::string msg;
		};

		void clear();
		void start_particles(nl::node src, int32_t type, int32_t direction, int32_t speed);
		void start_bursts(const std::string &path);
		void start_fireworks(const std::string &path);
		void show_bar(const std::string &msg, int32_t float_type);
		void restore_bgm() const;
		int64_t at(float alpha) const;

		int64_t now = 0;
		int64_t started = 0;
		bool from_map = false;
		uint8_t packet_flag = 0;
		bool mob_has_left = false;
		std::string bgm;
		std::string bgm_override;
		std::vector<Timed> timeline;

		std::vector<Texture> canvases;
		bool resize = true;
		int32_t sway = 100;
		int32_t direction = 0;
		std::vector<Drop> drops;
		std::vector<Show> shows;
		std::vector<Fireworks> fireworks;
		EffectLayer shots;
		Sound effect;

		Texture bar_left;
		Texture bar_tile;
		Texture bar_right;
		int16_t bar_y = 0;
		Text bar_text;
		int64_t bar_until = 0;
	};
}
