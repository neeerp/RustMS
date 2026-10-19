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
#include "MapWeather.h"

#include "../FieldEffectRules.h"

#include "../../Constants.h"

#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cmath>
#include <cstdlib>

namespace ms
{
	namespace
	{
		// The item data names its art from Map.nx's top ("Map/MapHelper.img/weather/snow").
		nl::node map_node(const std::string &path)
		{
			if (path.rfind("Map/", 0) != 0)
				return {};

			return nl::nx::map.resolve(path.substr(4));
		}

		// Frames "0", "1", ... up to the first missing one, 120 ms each unless they say otherwise (FUN_0043f768).
		Animation weather_animation(nl::node src)
		{
			std::vector<Frame> frames;

			for (nl::node frame = src["0"]; frame; frame = src[std::to_string(frames.size())])
			{
				uint16_t delay = frame["delay"];
				frames.emplace_back(frame, delay ? delay : weather::FRAME_DELAY, 255, 255);
			}

			if (frames.empty())
				return {};

			return Animation(std::move(frames));
		}

		int32_t roll()
		{
			return std::rand() & 0x7FFF;
		}

		// How far below the bar's text line our text's top sits.
		constexpr int16_t TEXT_TOP = -6;
	}

	MapWeather::MapWeather(nl::node src, std::string map_bgm) : bgm(std::move(map_bgm))
	{
		for (nl::node entry = src["0"]; entry; entry = src[std::to_string(timeline.size())])
		{
			nl::node item = entry["item"];
			// The data stores the item as a string ("05120014").
			int32_t id = item.data_type() == nl::node::type::string
						 ? string_conversion::or_default<int32_t>(item.get_string(), 0)
						 : item.get_integer();

			timeline.push_back({ entry["time"].get_integer() * 1000, id, entry["option"], entry["msg"] });
		}
	}

	MapWeather::MapWeather()
	{}

	void MapWeather::blow(int32_t item_id, const std::string &msg, bool map)
	{
		from_map = map;

		if (item_id == 0)
		{
			effect.stop();
			restore_bgm();

			// Each particle fades from where it is to nothing over its own 1 to 2 s (FUN_00641c28). Bursts and
			// fireworks go on until their 30 s are up.
			for (Drop &drop : drops)
			{
				if (drop.stopped >= 0)
					continue;

				drop.stop_alpha = weather::fade_in(drop.particle, now - drop.born, drop.animation.has_value());
				drop.stopped = now;
				drop.stop_length = weather::fade_out_time(roll());
			}

			if (packet_flag == 0)
				bar_until = 0;

			started = 0;
			return;
		}

		started = now;

		if (!weather::is_weather(item_id))
			return;

		nl::node info = nl::nx::item["Cash"]["0512.img"]["0" + std::to_string(item_id)]["info"];

		if (!info)
			return;

		std::string path = info["path"];

		if (path.empty())
			return;

		std::string sound = info["bgmPath"];

		if (!sound.empty())
		{
			// Sound.nx is the top of these paths ("Sound/CashEffect.img/05122000/Use").
			sound = field_effect::jukebox_path(sound);
			bool repeat = info["repeat"].get_integer() != 0;

			if (info["isBgmOrEffect"].get_integer() != 0)
			{
				Music(sound).play(1000, 0, 1000, repeat);
			}
			else
			{
				effect = Sound(nl::nx::sound.resolve(sound));

				if (repeat)
					effect.play_loop();
				else
					effect.play();
			}
		}

		if (!msg.empty())
			show_bar(msg, info["floatType"]);

		switch (info["type"].get_integer())
		{
			case 3:
				start_bursts(path);
				break;
			case 4:
				start_fireworks(path);
				break;
			default:
				start_particles(map_node(path), info["type"], info["direction"], info["speed"]);
				break;
		}
	}

	void MapWeather::set_packet_flag(uint8_t flag)
	{
		packet_flag = flag;
	}

	void MapWeather::mob_left()
	{
		mob_has_left = true;
	}

	void MapWeather::set_bgm_override(std::string path)
	{
		bgm_override = std::move(path);
	}

	void MapWeather::clear()
	{
		// v83 drops the old weather at once, without a fade (FUN_00641aeb). It leaves a firework's sound looping
		// with nothing to stop it, which is stopped here.
		drops.clear();
		shows.clear();

		for (Fireworks &show : fireworks)
			show.sound.stop();

		fireworks.clear();
	}

	void MapWeather::start_particles(nl::node src, int32_t type, int32_t direction, int32_t speed)
	{
		clear();

		bool animated = src["ani"].get_integer() != 0;
		bool random = src["random"].get_integer() != 0;
		int32_t count = src["count"].get_integer(random ? 100 : 0);
		count = weather::particle_count(random, count, random ? roll() : 0);
		resize = src["noresize"].get_integer() == 0;
		sway = static_cast<int32_t>(src["x"].get_integer(100));
		this->direction = direction;

		// Kairin has no particles: only its message shows.
		if (count <= 0)
			return;

		canvases.clear();
		std::vector<nl::node> animations;

		if (animated)
		{
			for (nl::node child = src["0"]; child; child = src[std::to_string(animations.size())])
				animations.push_back(child);
		}
		else
		{
			for (nl::node child : src)
				if (child.data_type() == nl::node::type::bitmap)
					canvases.emplace_back(child);
		}

		for (int32_t i = 0; i < count; i++)
		{
			Drop drop{ weather::particle(type, direction, speed, static_cast<int32_t>(canvases.size()),
										 static_cast<int32_t>(animations.size()), roll), std::nullopt, now };

			if (animated)
				drop.animation = weather_animation(animations[drop.particle.frame]);

			drops.push_back(std::move(drop));
		}
	}

	void MapWeather::start_bursts(const std::string &path)
	{
		clear();

		// "squib/squib" names squib0 to squib4; the items on "none" have no such nodes and show only their message.
		for (size_t i = 0; i < weather::BURSTS.size(); i++)
		{
			nl::node src = map_node(path + std::to_string(i));

			if (!src)
				continue;

			auto [interval, count] = weather::BURSTS[i];
			shows.push_back({ weather_animation(src), interval, count, now, now + weather::SHOW_TIME, {} });
		}
	}

	void MapWeather::start_fireworks(const std::string &path)
	{
		clear();

		nl::node src = map_node(path + "/0");

		if (!src)
			return;

		fireworks.push_back({ weather_animation(src), Sound(map_node(path + "/sound1")), false, now,
							  now + weather::SHOW_TIME });
	}

	void MapWeather::show_bar(const std::string &msg, int32_t float_type)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["FloatNotice"][std::to_string(float_type)];

		if (!src)
			return;

		bar_left = src["0"];
		bar_tile = src["1"];
		bar_right = src["2"];
		bar_y = src["y"];
		bar_text = Text(Text::Font::A14B, Text::Alignment::LEFT, Color::Name::WHITE, msg);
		bar_until = now + weather::BAR_TIME;
	}

	void MapWeather::restore_bgm() const
	{
		// The BGM a field effect chose comes back with 1 s fades; otherwise the map's, if it changed (FUN_00642214).
		if (bgm_override.empty())
			Music(bgm).play(field_effect::BGM_FADE);
		else
			Music(bgm_override).play(1000, 0, 1000);
	}

	void MapWeather::update()
	{
		now += Constants::TIMESTEP;

		// Option 1 only blows while no monster has left the field (CField::Update FUN_00531b8d).
		for (auto entry = timeline.begin(); entry != timeline.end();)
		{
			if (entry->option == 1 && mob_has_left)
			{
				entry = timeline.erase(entry);
			}
			else if (now >= entry->time)
			{
				Timed due = *entry;
				entry = timeline.erase(entry);
				blow(due.item, due.msg, true);
			}
			else
			{
				++entry;
			}
		}

		if (from_map && started && now - started > weather::MAP_TIME)
			blow(0, "", true);

		if (bar_until && now > bar_until)
			bar_until = 0;

		for (Drop &drop : drops)
			if (drop.animation)
				drop.animation->update();

		int16_t view_width = Constants::Constants::get().get_viewwidth();
		int16_t view_height = Constants::Constants::get().get_viewheight();
		Point<int16_t> centre(view_width / 2, view_height / 2);

		for (auto show = shows.begin(); show != shows.end();)
		{
			if (now > show->end)
			{
				show = shows.erase(show);
				continue;
			}

			if (now >= show->next)
			{
				show->next += show->interval;
				auto [range_x, range_y] = weather::spread(show->spread);

				for (int32_t i = 0; i < show->count; i++)
				{
					int32_t y = weather::burst_offset(range_y, static_cast<uint32_t>(roll()));
					int32_t x = weather::burst_offset(range_x, static_cast<uint32_t>(roll()));
					shots.add(show->animation, DrawArgument(Point<int16_t>(x, y)));
				}
			}

			++show;
		}

		for (auto show = fireworks.begin(); show != fireworks.end();)
		{
			if (now > show->end)
			{
				show->sound.stop();
				show = fireworks.erase(show);
				continue;
			}

			if (now >= show->next)
			{
				show->next += weather::FIREWORK_INTERVAL;

				for (int16_t x : weather::FIREWORK_X)
					shots.add(show->animation, DrawArgument(Point<int16_t>(x, weather::FIREWORK_Y)));

				if (!show->sounding)
				{
					show->sound.play_loop();
					show->sounding = true;
				}
			}

			++show;
		}

		shots.update();
	}

	int64_t MapWeather::at(float alpha) const
	{
		return std::max<int64_t>(0, now - Constants::TIMESTEP + std::lround(alpha * Constants::TIMESTEP));
	}

	void MapWeather::draw_shows(float alpha) const
	{
		int16_t view_width = Constants::Constants::get().get_viewwidth();
		int16_t view_height = Constants::Constants::get().get_viewheight();

		// Offsets from the screen's centre, which follow the camera.
		shots.drawabove(Point<int16_t>(view_width / 2, view_height / 2), alpha);
	}

	void MapWeather::draw(double viewx, double viewy, float alpha) const
	{
		if (drops.empty())
			return;

		int64_t time = at(alpha);
		int16_t view_width = Constants::Constants::get().get_viewwidth();
		int16_t view_height = Constants::Constants::get().get_viewheight();
		auto shift_x = static_cast<int32_t>(std::round(viewx));
		auto shift_y = static_cast<int32_t>(std::round(viewy));
		int32_t centre_x = view_width / 2 - shift_x;
		int32_t centre_y = view_height / 2 - shift_y;
		bool on_screen = weather::on_screen(direction);

		for (const Drop &drop : drops)
		{
			const weather::Particle &particle = drop.particle;
			int64_t age = time - drop.born;
			bool animated = drop.animation.has_value();
			float opacity = drop.stopped < 0
							? weather::fade_in(particle, age, animated)
							: weather::fade_out(drop.stop_alpha, time - drop.stopped, drop.stop_length);

			if (opacity <= 0.0f)
				continue;

			weather::Spot spot{};

			// Directions 4 and 5 fly across v83's 800x600 screen, here centred on the view.
			if (on_screen)
			{
				spot = weather::screen_spot(particle, age);
				spot.x += (view_width - 800) / 2;
				spot.y += (view_height - 600) / 2;
			}
			else
			{
				spot = weather::world_spot(particle, age, centre_x, centre_y, sway, view_width, view_height);
				spot.x += shift_x;
				spot.y += shift_y;
			}

			auto angle = static_cast<float>(weather::spin_angle(particle, age) * 3.14159265358979323846 / 180.0);
			opacity = std::min(opacity, 255.0f) / 255.0f;

			// v83 centres the sprite on its spot, ignoring the canvas origin.
			if (drop.animation)
			{
				Point<int16_t> size = drop.animation->get_dimensions();
				Point<int16_t> left_top(static_cast<int16_t>(spot.x - size.x() / 2),
										static_cast<int16_t>(spot.y - size.y() / 2));
				Point<int16_t> position = left_top + drop.animation->get_origin();
				drop.animation->draw(DrawArgument(position, position, Point<int16_t>(), 1.0f, 1.0f, opacity, angle),
									 alpha);
			}
			else if (particle.frame < static_cast<int32_t>(canvases.size()))
			{
				const Texture &canvas = canvases[particle.frame];
				Point<int16_t> stretch = resize ? Point<int16_t>(particle.size, particle.size) : Point<int16_t>();
				Point<int16_t> size = resize ? stretch : canvas.get_dimensions();
				Point<int16_t> left_top(static_cast<int16_t>(spot.x - size.x() / 2),
										static_cast<int16_t>(spot.y - size.y() / 2));
				Point<int16_t> position = left_top + canvas.get_origin();
				canvas.draw(DrawArgument(position, position, stretch, 1.0f, 1.0f, opacity, angle));
			}
		}
	}

	void MapWeather::draw_bar() const
	{
		if (!bar_until)
			return;

		int16_t view_width = Constants::Constants::get().get_viewwidth();
		weather::Bar bar = weather::bar(view_width, bar_left.width(), bar_tile.width(), bar_right.width(),
										bar_text.width());

		// The art's origins are ignored: each piece's top left goes at its spot.
		auto piece = [&](const Texture &texture, int16_t x) {
			Point<int16_t> left_top(static_cast<int16_t>(bar.left + x), weather::BAR_TOP);
			texture.draw(DrawArgument(left_top + texture.get_origin(), weather::BAR_ALPHA));
		};

		piece(bar_left, 0);

		for (int16_t i = 0; i < bar.tiles; i++)
			piece(bar_tile, static_cast<int16_t>(bar_left.width() + i * bar_tile.width()));

		piece(bar_right, static_cast<int16_t>(bar_left.width() + bar.tiles * bar_tile.width()));

		bar_text.draw(Point<int16_t>(static_cast<int16_t>(bar.left + bar.text_x),
									 static_cast<int16_t>(weather::BAR_TOP + bar_y + TEXT_TOP)));
	}

	void MapWeather::stop_sounds()
	{
		effect.stop();

		for (Fireworks &show : fireworks)
			show.sound.stop();
	}
}
