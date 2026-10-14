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
#include "MapNpcs.h"

#include "../QuestConversation.h"
#include "Npc.h"
#include "NpcReach.h"

#include "../Net/Packets/NpcInteractionPackets.h"

namespace ms
{
	void MapNpcs::draw(Layer::Id layer, double viewx, double viewy, float alpha) const
	{
		npcs.draw(layer, viewx, viewy, alpha);
	}

	void MapNpcs::update(const Physics &physics)
	{
		for (; !spawns.empty(); spawns.pop())
		{
			const NpcSpawn &spawn = spawns.front();

			int32_t oid = spawn.get_oid();
			Optional<MapObject> npc = npcs.get(oid);

			if (npc)
				npc->makeactive();
			else
				npcs.add(spawn.instantiate(physics));
		}

		npcs.update(physics);
	}

	void MapNpcs::spawn(NpcSpawn &&spawn)
	{
		spawns.emplace(std::move(spawn));
	}

	void MapNpcs::remove(int32_t oid)
	{
		if (auto npc = npcs.get(oid))
			npc->deactivate();
	}

	void MapNpcs::clear()
	{
		npcs.clear();
	}

	MapObjects *MapNpcs::get_npcs()
	{
		return &npcs;
	}

	Cursor::State MapNpcs::send_cursor(bool pressed, Point<int16_t> position, Point<int16_t> viewpos)
	{
		for (auto &map_object : npcs)
		{
			Npc *npc = static_cast<Npc *>(map_object.second.get());

			if (npc && npc->is_active() && npc->inrange(position, viewpos))
			{
				if (pressed)
				{
					// The client offers the NPC's quests itself; only plain speech
					// goes to the server.
					QuestConversation::talk_to_npc(npc->get_npcid(), npc->get_oid());

					return Cursor::State::IDLE;
				} else
				{
					return Cursor::State::CANCLICK;
				}
			}
		}

		return Cursor::State::IDLE;
	}

	Npc *MapNpcs::find_nearest(Point<int16_t> position)
	{
		Npc *nearest = nullptr;

		for (auto &map_object : npcs)
		{
			Npc *npc = static_cast<Npc *>(map_object.second.get());

			if (!npc || !npc->is_active() || !npc_reach::in_reach(position, npc->get_position()))
				continue;

			if (!nearest || npc_reach::nearer(position, npc->get_position(), nearest->get_position()))
				nearest = npc;
		}

		return nearest;
	}
}
