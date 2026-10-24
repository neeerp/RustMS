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
#include "CharEquips.h"

#include "CashWeapon.h"

namespace ms
{
	CharEquips::CharEquips()
	{
		for (auto iter : clothes)
			iter.second = nullptr;
	}

	void CharEquips::draw(EquipSlot::Id slot, Stance::Id stance, Clothing::Layer layer, uint8_t frame, const DrawArgument& args) const
	{
		if (riding && (slot == EquipSlot::Id::WEAPON || slot == EquipSlot::Id::SHIELD))
			return;

		// A cash weapon takes the weapon's place, even with no art for it.
		if (slot == EquipSlot::Id::WEAPON && cash_weapon)
		{
			if (sticker)
				sticker->draw(stance, layer, frame, args);

			return;
		}

		if (const Clothing* cloth = clothes[slot])
			cloth->draw(stance, layer, frame, args);
	}

	void CharEquips::add_equip(int32_t itemid, const BodyDrawInfo& drawinfo)
	{
		if (itemid <= 0)
			return;

		auto iter = cloth_cache.find(itemid);

		if (iter == cloth_cache.end())
		{
			iter = cloth_cache.emplace(
					std::piecewise_construct,
					std::forward_as_tuple(itemid),
					std::forward_as_tuple(itemid, drawinfo)
			).first;
		}

		const Clothing &cloth = iter->second;

		EquipSlot::Id slot = cloth.get_eqslot();
		clothes[slot] = &cloth;

		if (slot == EquipSlot::Id::WEAPON)
			update_sticker(drawinfo);
	}

	void CharEquips::remove_equip(EquipSlot::Id slot)
	{
		clothes[slot] = nullptr;

		if (slot == EquipSlot::Id::WEAPON)
			sticker = nullptr;
	}

	void CharEquips::set_cash_weapon(int32_t itemid, const BodyDrawInfo& drawinfo)
	{
		cash_weapon = itemid > 0 ? itemid : 0;
		update_sticker(drawinfo);
	}

	int32_t CharEquips::get_cash_weapon() const
	{
		return cash_weapon;
	}

	void CharEquips::update_sticker(const BodyDrawInfo& drawinfo)
	{
		sticker = nullptr;

		int32_t weapon = get_weapon();
		int32_t type = cash_weapon::type(weapon);

		if (!cash_weapon || !type)
			return;

		auto key = std::make_pair(cash_weapon, type);
		auto iter = sticker_cache.find(key);

		if (iter == sticker_cache.end())
		{
			iter = sticker_cache.emplace(
					std::piecewise_construct,
					std::forward_as_tuple(key),
					std::forward_as_tuple(cash_weapon, weapon, drawinfo)
			).first;
		}

		sticker = &iter->second;
	}

	bool CharEquips::is_visible(EquipSlot::Id slot) const
	{
		if (const Clothing *cloth = clothes[slot])
			return cloth->is_transparent() == false;
		else
			return false;
	}

	bool CharEquips::comparelayer(EquipSlot::Id slot, Stance::Id stance, Clothing::Layer layer) const
	{
		if (const Clothing *cloth = clothes[slot])
			return cloth->contains_layer(stance, layer);
		else
			return false;
	}

	bool CharEquips::has_overall() const
	{
		return get_equip(EquipSlot::Id::TOP) / 10000 == 105;
	}

	bool CharEquips::has_weapon() const
	{
		return get_weapon() != 0;
	}

	bool CharEquips::is_twohanded() const
	{
		if (const Clothing* weapon = clothes[EquipSlot::Id::WEAPON])
			return weapon->is_twohanded();
		else
			return false;
	}

	CharEquips::CapType CharEquips::getcaptype() const
	{
		if (const Clothing* cap = clothes[EquipSlot::Id::HAT])
		{
			const std::string &vslot = cap->get_vslot();
			if (vslot == "CpH1H5")
				return CharEquips::CapType::HALFCOVER;
			else if (vslot == "CpH1H5AyAs")
				return CharEquips::CapType::FULLCOVER;
			else if (vslot == "CpH5")
				return CharEquips::CapType::HEADBAND;
			else
				return CharEquips::CapType::NONE;
		} else
		{
			return CharEquips::CapType::NONE;
		}
	}

	Stance::Id CharEquips::adjust_stance(Stance::Id stance) const
	{
		if (const Clothing* weapon = clothes[EquipSlot::Id::WEAPON])
		{
			switch (stance)
			{
				case Stance::Id::STAND1:
				case Stance::Id::STAND2:
					return weapon->get_stand();
				case Stance::Id::WALK1:
				case Stance::Id::WALK2:
					return weapon->get_walk();
				default:
					return stance;
			}
		} else
		{
			return stance;
		}
	}

	int32_t CharEquips::get_equip(EquipSlot::Id slot) const
	{
		if (const Clothing *cloth = clothes[slot])
			return cloth->get_id();
		else
			return 0;
	}

	int32_t CharEquips::get_weapon() const
	{
		return get_equip(EquipSlot::Id::WEAPON);
	}

	void CharEquips::set_riding(bool r)
	{
		riding = r;
	}

	std::unordered_map<int32_t, Clothing> CharEquips::cloth_cache;
	std::map<std::pair<int32_t, int32_t>, Clothing> CharEquips::sticker_cache;
}