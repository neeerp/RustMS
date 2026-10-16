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

#include <unordered_map>
#include <cstdint>
#include <string>
#include <array>
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
			TOMBSTONE,
			LENGTH
		};

		Sound(Name name);

		Sound(int32_t itemid);

		Sound(nl::node src);

		Sound();
		//~Sound();

		void play();

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
	};

	class Music
	{
	public:
		Music(std::string path);

		void play() const;

		void play_once() const;

		static Error init();

		// Volume from 0 to 100.
		static bool set_bgmvolume(uint8_t volume);

		// The volume from the settings, 0 when muted.
		static uint8_t configured_volume();

		static void update_context();

	private:
		std::string path;
		static std::unordered_map<std::string, membuf *> audiodb;
		static alure::DeviceManager devMgr;
		static alure::Device dev;
		static alure::Context ctx;
		static alure::Source music_src;
		static alure::Buffer music_buff;

		friend Sound;
	};
}
