//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton, lain3d				//
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

#include <cstddef>

#include "../Error.h"

#include "../Template/EnumMap.h"
#include "../Template/Point.h"

#include <unordered_map>
#include <cstdint>
#include <string>
#include <array>
#include <chrono>
#include <vector>

#include <nlnx/node.hpp>
#include <AL/al.h>
#include <AL/alure2.h>
#include "membuf.h"

class FileFactory final : public alure::FileIOFactory
{
private:
	std::unordered_map<std::string, membuf *> *audiodb;
public:
	FileFactory(std::unordered_map<std::string, membuf *> *audiodb_in) : audiodb(audiodb_in)
	{}

	alure::UniquePtr<std::istream> openFile(const alure::String &name) noexcept override
	{
		auto stream = alure::MakeUnique<std::istream>(audiodb->at(name));
		if (stream->fail())
			return {};
		return stream;
	}
};

namespace ms
{
	class Sound
	{
	public:
		// Preloaded sounds
		enum Name
		{
			/// UI
			BUTTONCLICK,
			BUTTONOVER,
			CHARSELECT,
			DLGNOTICE,
			INVITE,
			MENUDOWN,
			MENUUP,
			RACESELECT,
			SCROLLUP,
			SELECTMAP,
			TAB,
			WORLDSELECT,
			DRAGSTART,
			DRAGEND,
			WORLDMAPOPEN,
			WORLDMAPCLOSE,
			ROLLDOWN,
			ROLLUP,
			BUYSHOPITEM,

			/// Login
			GAMESTART,

			/// Game
			JUMP,
			DROP,
			PICKUP,
			PORTAL,
			LEVELUP,
			JOBCHANGED,
			QUESTCLEAR,
			QUESTALERT,
			TOMBSTONE,
			ENCHANTSUCCESS,
			ENCHANTFAILURE,
			MCARDGET,
			ITEMLEVELUP,
			INCEXP,
			LENGTH
		};

		Sound(Name name);

		Sound(int32_t itemid);

		Sound(nl::node src);

		Sound();
		//~Sound();

		void play();

		// Play at `volume` percent of the sound effects' volume.
		void play(uint8_t volume);

		// Play a sound made at `source` in the field, quieter the farther it
		// is from the player (FUN_009894f3).
		void play_from(Point<int16_t> source);

		// Where the player stands, for play_from; none while there is no player.
		static void set_listener(Point<int16_t> position);
		static void clear_listener();

		// Play the sound over and over until stop.
		void play_loop();

		// Stop the sound play_loop started.
		void stop();

		static Error init();

		static void close();

		// Volume from 0 to 100.
		static bool set_sfxvolume(uint8_t volume);

		// The volume from the settings, 0 when muted.
		static uint8_t configured_volume();

	private:
		size_t id;
		alure::Source looping;


		static size_t add_sound(nl::node src);

		static void add_sound(Name name, nl::node src);

		static void add_sound(std::string itemid, nl::node src);

		static alure::Source free_source();

		static std::string format_id(int32_t itemid);

		static EnumMap<Name, size_t> soundids;
		static std::unordered_map<std::string, size_t> itemids;
		static constexpr size_t MAX_SOURCES = 100;
		static std::vector<alure::Source> sources;
		static float gain;
		static Point<int16_t> listener;
		static bool has_listener;
	};

	class Music
	{
	public:
		Music(std::string path);

		// Play this track, looping unless told not to. With a fade, the track playing fades out over that many ms
		// first. A delay holds the start back that many ms from now, and a fade-in raises the track from silence over
		// that many ms.
		void play(int32_t fadeout = 0, int32_t delay = 0, int32_t fadein = 0, bool loop = true) const;

		void play_once() const;

		static Error init();

		// Volume from 0 to 100.
		static bool set_bgmvolume(uint8_t volume);

		// The volume from the settings, 0 when muted.
		static uint8_t configured_volume();

		static void update_context();

	private:
		std::string path;

		static void start(const std::string &path);

		static std::unordered_map<std::string, membuf *> audiodb;
		static alure::DeviceManager devMgr;
		static alure::Device dev;
		static alure::Context ctx;
		static alure::Source music_src;
		static alure::Buffer music_buff;
		static float volume;
		// The track to start once the one playing has faded out.
		static std::string pending;
		static std::chrono::steady_clock::time_point fade_start;
		static int32_t fade_length;
		// The pending track starts this many ms after fade_start at the soonest.
		static int32_t start_delay;
		// The next track to start rises from silence over this many ms.
		static int32_t fadein_length;
		// Whether the next track to start loops.
		static bool looping;
		static bool fading_in;
		static std::chrono::steady_clock::time_point fadein_start;

		friend Sound;
	};
}
