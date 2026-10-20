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

#include <string>

#include <cstdint>

#include "MapleMap/MapObject.h"

#include "../Character/Buff.h"
#include "../Character/Guild.h"
#include "../Character/MiniRoomBox.h"
#include "../Graphics/Animation.h"
#include "../Net/Login.h"
#include "../Net/Handlers/Helpers/MobStatParser.h"
#include "../Net/Handlers/Helpers/PetParser.h"

#include <memory>
#include <optional>
#include <utility>
#include <vector>

namespace ms
{
	class NpcSpawn
	{
	public:
		NpcSpawn(int32_t oid, int32_t npcid, Point<int16_t> position, bool mirrored, uint16_t fh);

		int32_t get_oid() const;

		std::unique_ptr<MapObject> instantiate(const Physics &physics) const;

	private:
		int32_t oid;
		int32_t id;
		Point<int16_t> position;
		bool flip;
		uint16_t fh;
	};

	class MobSpawn
	{
	public:
		// `summon` is how the mob appears: the Effect/Summon.img effect a
		// summoned mob plays `summondelay` ms in, or a negative type.
		MobSpawn(int32_t oid, int32_t id, int8_t mode, int8_t stance, uint16_t fh, uint16_t homefh, bool newspawn,
				 int8_t team, Point<int16_t> position, std::vector<MobStatParser::Status> statuses,
				 int8_t summon = -1, int32_t summondelay = 0);

		int8_t get_mode() const;

		int32_t get_oid() const;

		std::unique_ptr<MapObject> instantiate(const Physics &physics) const;

	private:
		int32_t oid;
		int32_t id;
		int8_t mode;
		int8_t stance;
		uint16_t fh;
		uint16_t homefh;
		bool newspawn;
		int8_t team;
		Point<int16_t> position;
		std::vector<MobStatParser::Status> statuses;
		int8_t summon;
		int32_t summondelay;
	};

	class ReactorSpawn
	{
	public:
		ReactorSpawn(int32_t oid, int32_t rid, int8_t state, Point<int16_t> position);

		int32_t get_oid() const;

		std::unique_ptr<MapObject> instantiate(const Physics &physics) const;

	private:
		int32_t oid;
		int32_t rid;
		int8_t state;
		Point<int16_t> position;
	};

	class DropSpawn
	{
	public:
		DropSpawn(int32_t oid, int32_t id, bool meso, int32_t owner, Point<int16_t> position,
				  Point<int16_t> destination, int8_t droptype, int8_t mode, int16_t delay, bool playerdrop);

		bool is_meso() const;

		int32_t get_itemid() const;

		int32_t get_oid() const;

		std::unique_ptr<MapObject> instantiate(const Animation &icon) const;

		std::unique_ptr<MapObject> instantiate(const Texture &icon) const;

	private:
		int32_t oid;
		int32_t id;
		bool meso;
		int32_t owner;
		Point<int16_t> start;
		Point<int16_t> dest;
		int8_t droptype;
		int8_t mode;
		int16_t delay;
		bool playerdrop;
	};

	class CharSpawn
	{
	public:
		CharSpawn(int32_t cid, const LookEntry &look, uint8_t level, int16_t job, const std::string &name,
				  int8_t stance, Point<int16_t> position, int32_t chair,
				  std::vector<std::pair<Buffstat::Id, int32_t>> buffs, const std::string &guild,
				  const GuildEmblem &emblem, std::optional<MiniRoomBox> room = std::nullopt,
				  std::vector<PetEntry> pets = {}, std::optional<std::string> chalkboard = std::nullopt,
				  int32_t item_effect = 0, int32_t mount_level = 0, int32_t mount_exp = 0,
				  int32_t mount_fatigue = 0);

		int32_t get_cid() const;

		std::unique_ptr<MapObject> instantiate() const;

	private:
		int32_t cid;
		uint8_t level;
		int16_t job;
		std::string name;
		int8_t stance;
		Point<int16_t> position;
		int32_t chair;
		LookEntry look;
		std::vector<std::pair<Buffstat::Id, int32_t>> buffs;
		std::string guild;
		GuildEmblem emblem;
		std::optional<MiniRoomBox> room;
		std::vector<PetEntry> pets;
		std::optional<std::string> chalkboard;
		int32_t item_effect;
		int32_t mount_level;
		int32_t mount_exp;
		int32_t mount_fatigue;
	};
}