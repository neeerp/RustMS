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
#include "MapDrops.h"

#include "Drop.h"
#include "Pickup.h"
#include "ItemDrop.h"
#include "MesoDrop.h"

#include "../Constants.h"

#include "../Data/ItemData.h"

#include "../../Util/Randomizer.h"

#include <algorithm>

#include <nlnx/nx.hpp>
#include <nlnx/node.hpp>

namespace ms
{
	MapDrops::MapDrops() : sinceblast(meso_explosion::SOUND_GAP) {}

	void MapDrops::init()
	{
		nl::node src = nl::nx::item["Special"]["0900.img"];

		mesoicons[MesoIcon::BRONZE] = src["09000000"]["iconRaw"];
		mesoicons[MesoIcon::GOLD] = src["09000001"]["iconRaw"];
		mesoicons[MesoIcon::BUNDLE] = src["09000002"]["iconRaw"];
		mesoicons[MesoIcon::BAG] = src["09000003"]["iconRaw"];

		std::string id = std::to_string(meso_explosion::ID);

		for (auto sub : nl::nx::skill[id.substr(0, 3) + ".img"]["skill"][id]["hit"])
			blasts.emplace_back(sub);

		blastsound = nl::nx::sound["Skill.img"][id]["Hit"];
	}

	void MapDrops::draw(Layer::Id layer, double viewx, double viewy, float alpha) const
	{
		drops.draw(layer, viewx, viewy, alpha);
	}

	void MapDrops::draw_explosions(Point<int16_t> viewpos, float alpha) const
	{
		explosions.drawabove(viewpos, alpha);
	}

	void MapDrops::update(const Physics &physics)
	{
		for (; !spawns.empty(); spawns.pop())
		{
			const DropSpawn &spawn = spawns.front();

			int32_t oid = spawn.get_oid();

			if (Optional<MapObject> drop = drops.get(oid))
			{
				drop->makeactive();
			} else
			{
				int32_t itemid = spawn.get_itemid();
				bool meso = spawn.is_meso();

				if (meso)
				{
					MesoIcon mesotype = (itemid > 999)
										? BAG : (itemid > 99)
												? BUNDLE : (itemid > 49)
														   ? GOLD : BRONZE;

					const Animation &icon = mesoicons[mesotype];
					drops.add(spawn.instantiate(icon));
				} else if (const ItemData &itemdata = ItemData::get(itemid))
				{
					const Texture &icon = itemdata.get_icon(true);
					drops.add(spawn.instantiate(icon));
				}
			}
		}

		for (auto &mesoicon : mesoicons)
			mesoicon.update();

		drops.update(physics);

		// A drop that went off shows one of the skill's hits where it lay,
		// and sounds, but not again within 90 ms.
		sinceblast = std::min(sinceblast + Constants::TIMESTEP, meso_explosion::SOUND_GAP);

		for (auto &mmo : drops)
		{
			Optional<Drop> drop = mmo.second.get();

			if (!drop || !drop->went_off())
				continue;

			if (!blasts.empty())
				explosions.add(blasts[Randomizer().next_int(blasts.size())], DrawArgument(drop->get_spot()), 0);

			if (sinceblast >= meso_explosion::SOUND_GAP)
			{
				blastsound.play();
				sinceblast = 0;
			}
		}

		explosions.update();
	}

	void MapDrops::spawn(DropSpawn &&spawn)
	{
		spawns.emplace(std::move(spawn));
	}

	void MapDrops::remove(int32_t oid, int8_t mode, const PhysicsObject *looter)
	{
		if (Optional<Drop> drop = drops.get(oid))
			drop->expire(mode, looter);
	}

	void MapDrops::explode(int32_t oid, int16_t delay)
	{
		if (Optional<Drop> drop = drops.get(oid))
			drop->explode(delay);
	}

	std::vector<meso_explosion::Drop> MapDrops::find_explodable(Rectangle<int16_t> area, int32_t self,
																 int32_t party) const
	{
		std::vector<std::pair<uint32_t, meso_explosion::Drop>> found;

		for (auto &mmo : drops)
		{
			auto drop = static_cast<const Drop *>(mmo.second.get());

			if (drop && drop->explodable(area, self, party))
				found.push_back({ drops.entry_order(mmo.first), { mmo.first, drop->get_mesos(), drop->get_spot() } });
		}

		std::sort(found.begin(), found.end(), [](auto &a, auto &b) { return a.first < b.first; });

		std::vector<meso_explosion::Drop> explodable;

		for (auto &[entry, drop] : found)
			explodable.push_back(drop);

		return explodable;
	}

	void MapDrops::clear()
	{
		drops.clear();
		explosions.clear();
	}

	MapDrops::Loot MapDrops::find_loot_at(Point<int16_t> playerpos, int32_t self, int32_t party)
	{
		for (auto &mmo : drops)
		{
			Optional<Drop> drop = mmo.second.get();

			if (drop && drop->lootable(playerpos, self, party))
			{
				drop->try_loot();

				int32_t oid = mmo.first;
				Point<int16_t> position = drop->get_position();

				return {oid, position};
			}
		}

		return {0, {}};
	}

	int32_t MapDrops::find_pet_loot_at(Point<int16_t> petpos, int32_t self, int32_t party, bool mesos, bool items,
									   bool others, const std::vector<int32_t> &ignored)
	{
		for (auto &mmo : drops)
		{
			Optional<Drop> drop = mmo.second.get();

			if (drop && drop->pet_lootable(petpos, self, party, mesos, items, others, ignored))
			{
				drop->try_pet_loot();

				return mmo.first;
			}
		}

		return 0;
	}

	std::array<int8_t, 3> MapDrops::sweep(Point<int16_t> owner, const std::array<std::optional<Sweeper>, 3> &pets,
										  int32_t self, int32_t party)
	{
		std::array<int8_t, 3> directions = {0, 0, 0};
		std::array<bool, 3> free;
		std::array<int16_t, 3> petx = {0, 0, 0};

		for (size_t i = 0; i < 3; i++)
		{
			free[i] = pets[i].has_value();

			if (pets[i])
				petx[i] = pets[i]->position.x();
		}

		for (auto &mmo : drops)
		{
			if (std::none_of(free.begin(), free.end(), [](bool f) { return f; }))
				break;

			Optional<Drop> drop = mmo.second.get();

			if (!drop)
				continue;

			std::array<bool, 3> can = {false, false, false};

			for (size_t i = 0; i < 3; i++)
				can[i] = free[i] && drop->pet_sweepable(owner, pets[i]->position, self, party, pets[i]->items,
														pets[i]->others, pets[i]->longrange, *pets[i]->ignored);

			int8_t pet = pickup::sweeper_for(petx, can, drop->get_spot().x());

			if (pet < 0)
				continue;

			directions[pet] = pickup::sweep_direction(drop->get_spot().x(), petx[pet]);
			free[pet] = false;
		}

		return directions;
	}
}