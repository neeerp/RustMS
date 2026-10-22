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
#include "Inventory.h"
#include "ItemExpiry.h"

#include "../../Data/BulletData.h"
#include "../../Data/EquipData.h"
#include "../../Data/ItemData.h"

#include <algorithm>
#include <iostream>

namespace ms
{
	Inventory::Inventory()
	{
		bulletslot = 0;
		bulletprefix = 0;
		bulletlevel = 0;
		meso = 0;
		running_uid = 0;
		slotmaxima[InventoryType::Id::EQUIPPED] = EquipSlot::Id::LENGTH;
	}

	void Inventory::recalc_stats(Weapon::Type type, int32_t level)
	{
		totalstats.clear();

		for (auto &iter : inventories[InventoryType::Id::EQUIPPED])
		{
			auto equip_iter = equips.find(iter.second.unique_id);

			if (equip_iter != equips.end())
			{
				const Equip &equip = equip_iter->second;

				for (auto stat_iter : totalstats)
					stat_iter.second += equip.get_stat(stat_iter.first);
			}
		}

		int32_t prefix;

		switch (type)
		{
			case Weapon::Type::BOW:
				prefix = 2060;
				break;
			case Weapon::Type::CROSSBOW:
				prefix = 2061;
				break;
			case Weapon::Type::CLAW:
				prefix = 2070;
				break;
			case Weapon::Type::GUN:
				prefix = 2330;
				break;
			default:
				prefix = 0;
				break;
		}

		bulletprefix = prefix;
		bulletlevel = level;
		bulletslot = 0;

		if (prefix)
		{
			for (auto &iter : inventories[InventoryType::Id::USE])
			{
				const Slot &slot = iter.second;

				if (slot.count && is_bullet(slot))
				{
					bulletslot = iter.first;
					break;
				}
			}
		}

		if (int32_t bulletid = get_bulletid())
			totalstats[EquipStat::Id::WATK] += BulletData::get(bulletid).get_watk();
	}

	void Inventory::set_meso(int64_t m)
	{
		meso = m;
	}

	void Inventory::set_slotmax(InventoryType::Id type, uint8_t slotmax)
	{
		slotmaxima[type] = slotmax;
	}

	void Inventory::add_item(InventoryType::Id invtype, int16_t slot, int32_t item_id, int64_t cash_id, int64_t expire,
							 uint16_t count, const std::string &owner, int16_t flags)
	{
		items.emplace(
				std::piecewise_construct,
				std::forward_as_tuple(add_slot(invtype, slot, item_id, count, cash_id)),
				std::forward_as_tuple(item_id, expire, owner, flags)
		);
	}

	void Inventory::add_pet(InventoryType::Id invtype, int16_t slot, int32_t item_id, int64_t cash_id, int64_t expire,
							const std::string &name, int8_t level, int16_t closeness, int8_t fullness, int64_t date_dead,
							int32_t remain_life, uint16_t skill)
	{
		pets.emplace(
				std::piecewise_construct,
				std::forward_as_tuple(add_slot(invtype, slot, item_id, 1, cash_id)),
				std::forward_as_tuple(item_id, expire, name, level, closeness, fullness, date_dead, remain_life, skill)
		);
	}

	void Inventory::add_equip(InventoryType::Id invtype, int16_t slot, int32_t item_id, int64_t cash_id, int64_t expire,
							  uint8_t slots, uint8_t level, const EnumMap<EquipStat::Id, uint16_t> &stats,
							  const std::string &owner, int16_t flag, uint8_t ilevel, uint16_t iexp, int32_t vicious)
	{
		equips.emplace(
				std::piecewise_construct,
				std::forward_as_tuple(add_slot(invtype, slot, item_id, 1, cash_id)),
				std::forward_as_tuple(item_id, expire, owner, flag, slots, level, stats, ilevel, iexp, vicious)
		);
	}

	void Inventory::remove(InventoryType::Id type, int16_t slot)
	{
		auto iter = inventories[type].find(slot);

		if (iter == inventories[type].end())
			return;

		int32_t unique_id = iter->second.unique_id;
		inventories[type].erase(iter);

		switch (type)
		{
			case InventoryType::Id::EQUIPPED:
			case InventoryType::Id::EQUIP:
				equips.erase(unique_id);
				break;
			case InventoryType::Id::CASH:
				items.erase(unique_id);
				pets.erase(unique_id);
				break;
			default:
				items.erase(unique_id);
				break;
		}
	}

	void
	Inventory::swap(InventoryType::Id firsttype, int16_t firstslot, InventoryType::Id secondtype, int16_t secondslot)
	{
		Slot first = std::move(inventories[firsttype][firstslot]);
		inventories[firsttype][firstslot] = std::move(inventories[secondtype][secondslot]);
		inventories[secondtype][secondslot] = std::move(first);

		if (!inventories[firsttype][firstslot].item_id)
			remove(firsttype, firstslot);

		if (!inventories[secondtype][secondslot].item_id)
			remove(secondtype, secondslot);
	}

	int32_t Inventory::add_slot(InventoryType::Id type, int16_t slot, int32_t item_id, int16_t count, int64_t cash_id)
	{
		running_uid++;
		inventories[type][slot] = {running_uid, item_id, count, cash_id};

		return running_uid;
	}

	void Inventory::change_count(InventoryType::Id type, int16_t slot, int16_t count)
	{
		auto iter = inventories[type].find(slot);

		if (iter != inventories[type].end())
			iter->second.count = count;
	}

	void Inventory::modify(InventoryType::Id type, int16_t slot, int8_t mode, int16_t arg, Movement move)
	{
		if (slot < 0)
		{
			slot = -slot;
			type = InventoryType::Id::EQUIPPED;
		}

		arg = (arg < 0) ? -arg : arg;

		switch (mode)
		{
			case Modification::CHANGECOUNT:
				change_count(type, slot, arg);
				break;
			case Modification::SWAP:
				switch (move)
				{
					case Movement::MOVE_INTERNAL:
						swap(type, slot, type, arg);
						break;
					case Movement::MOVE_UNEQUIP:
						swap(InventoryType::Id::EQUIPPED, slot, InventoryType::Id::EQUIP, arg);
						break;
					case Movement::MOVE_EQUIP:
						swap(InventoryType::Id::EQUIP, slot, InventoryType::Id::EQUIPPED, arg);
						break;
				}

				break;
			case Modification::REMOVE:
				remove(type, slot);
				break;
		}
	}

	uint8_t Inventory::get_slotmax(InventoryType::Id type) const
	{
		return slotmaxima[type];
	}

	uint16_t Inventory::get_stat(EquipStat::Id type) const
	{
		return totalstats[type];
	}

	int64_t Inventory::get_meso() const
	{
		return meso;
	}

	bool Inventory::has_projectile() const
	{
		return bulletslot > 0;
	}

	bool Inventory::has_equipped(EquipSlot::Id slot) const
	{
		return inventories[InventoryType::Id::EQUIPPED].count(slot) > 0;
	}

	int16_t Inventory::get_bulletslot() const
	{
		return bulletslot;
	}

	uint16_t Inventory::get_bulletcount() const
	{
		return get_item_count(InventoryType::Id::USE, bulletslot);
	}

	int32_t Inventory::get_bulletid() const
	{
		return get_item_id(InventoryType::Id::USE, bulletslot);
	}

	int16_t Inventory::find_bulletslot(int16_t cost) const
	{
		if (!bulletprefix)
			return 0;

		for (auto &iter : inventories[InventoryType::Id::USE])
		{
			const Slot &slot = iter.second;

			if (slot.count >= std::max<int16_t>(cost, 1) && is_bullet(slot))
				return iter.first;
		}

		return 0;
	}

	uint16_t Inventory::get_most_bullets() const
	{
		uint16_t most = 0;

		if (bulletprefix)
			for (auto &iter : inventories[InventoryType::Id::USE])
				if (is_bullet(iter.second))
					most = std::max<uint16_t>(most, iter.second.count);

		return most;
	}

	bool Inventory::is_bullet(const Slot &slot) const
	{
		return slot.item_id / 1000 == bulletprefix && BulletData::get(slot.item_id).get_reqlevel() <= bulletlevel;
	}

	EquipSlot::Id Inventory::find_equipslot(int32_t itemid) const
	{
		const EquipData &cloth = EquipData::get(itemid);

		if (!cloth.is_valid())
			return EquipSlot::Id::NONE;

		EquipSlot::Id eqslot = cloth.get_eqslot();

		if (eqslot == EquipSlot::Id::RING1)
		{
			// A cash ring looks for a free slot among the cash rings.
			int16_t layer = ItemData::get(itemid).is_cashitem() ? CASH_EQUIP : 0;

			for (EquipSlot::Id ring : { EquipSlot::Id::RING2, EquipSlot::Id::RING3, EquipSlot::Id::RING4 })
				if (!inventories[InventoryType::Id::EQUIPPED].count(ring + layer))
					return ring;

			return EquipSlot::Id::RING1;
		} else
		{
			return eqslot;
		}
	}

	int16_t Inventory::equip_position(int16_t slot, EquipSlot::Id part) const
	{
		return get_cash_id(InventoryType::Id::EQUIP, slot) ? part + CASH_EQUIP : part;
	}

	int16_t Inventory::shown_equip(EquipSlot::Id part) const
	{
		int16_t cash = part + CASH_EQUIP;

		return inventories[InventoryType::Id::EQUIPPED].count(cash) ? cash : part;
	}

	int16_t Inventory::find_free_slot(InventoryType::Id type) const
	{
		int16_t counter = 1;

		for (auto &iter : inventories[type])
		{
			if (iter.first != counter)
				return counter;

			counter++;
		}

		return counter <= slotmaxima[type] ? counter : 0;
	}

	int16_t Inventory::find_item(InventoryType::Id type, int32_t itemid) const
	{
		for (auto &iter : inventories[type])
			if (iter.second.item_id == itemid)
				return iter.first;

		return 0;
	}

	int16_t Inventory::find_unsealed(InventoryType::Id type, int32_t itemid) const
	{
		for (auto &iter : inventories[type])
			if (iter.second.item_id == itemid && !is_sealed(type, iter.first))
				return iter.first;

		return 0;
	}

	int16_t Inventory::count_slots(InventoryType::Id type, int32_t itemid) const
	{
		int16_t slots = 0;

		for (auto &iter : inventories[type])
			if (iter.second.item_id == itemid)
				slots++;

		return slots;
	}

	int16_t Inventory::get_item_count(InventoryType::Id type, int16_t slot) const
	{
		auto iter = inventories[type].find(slot);

		if (iter != inventories[type].end())
			return iter->second.count;
		else
			return 0;
	}

	int16_t Inventory::get_total_item_count(int32_t itemid) const
	{
		InventoryType::Id type = InventoryType::by_item_id(itemid);

		int16_t total_count = 0;

		for (auto &iter : inventories[type])
			if (iter.second.item_id == itemid)
				total_count += iter.second.count;

		return total_count;
	}

	int32_t Inventory::get_item_id(InventoryType::Id type, int16_t slot) const
	{
		auto iter = inventories[type].find(slot);

		if (iter != inventories[type].end())
			return iter->second.item_id;
		else
			return 0;
	}

	int64_t Inventory::get_cash_id(InventoryType::Id type, int16_t slot) const
	{
		auto iter = inventories[type].find(slot);

		if (iter != inventories[type].end())
			return iter->second.cash_id;
		else
			return 0;
	}

	int16_t Inventory::find_cash_item(InventoryType::Id type, int64_t cash_id) const
	{
		for (auto& iter : inventories[type])
			if (cash_id && iter.second.cash_id == cash_id)
				return iter.first;

		return 0;
	}

	Optional<const Pet> Inventory::get_pet(int64_t cash_id) const
	{
		auto slot_iter = inventories[InventoryType::Id::CASH].find(find_cash_item(InventoryType::Id::CASH, cash_id));

		if (slot_iter == inventories[InventoryType::Id::CASH].end())
			return {};

		auto pet_iter = pets.find(slot_iter->second.unique_id);

		return pet_iter == pets.end() ? nullptr : &pet_iter->second;
	}

	int16_t Inventory::get_pet_level(int64_t cash_id) const
	{
		Optional<const Pet> pet = get_pet(cash_id);

		return pet ? pet->get_level() : -1;
	}

	Optional<const Equip> Inventory::get_equip(InventoryType::Id type, int16_t slot) const
	{
		if (type != InventoryType::Id::EQUIPPED && type != InventoryType::Id::EQUIP)
			return {};

		auto slot_iter = inventories[type].find(slot);

		if (slot_iter == inventories[type].end())
			return {};

		auto equip_iter = equips.find(slot_iter->second.unique_id);

		if (equip_iter == equips.end())
			return {};

		return equip_iter->second;
	}

	Optional<const Item> Inventory::get_item(InventoryType::Id type, int16_t slot) const
	{
		auto slot_iter = inventories[type].find(slot);

		if (slot_iter == inventories[type].end())
			return {};

		auto item_iter = items.find(slot_iter->second.unique_id);

		if (item_iter == items.end())
			return {};

		return item_iter->second;
	}

	bool Inventory::is_sealed(InventoryType::Id type, int16_t slot) const
	{
		int16_t flags = 0;

		if (Optional<const Equip> equip = get_equip(type, slot))
			flags = equip->get_flags();
		else if (Optional<const Item> item = get_item(type, slot))
			flags = item->get_flags();

		return flags & item_expiry::SEALED;
	}

	Inventory::Movement Inventory::movementbyvalue(int8_t value)
	{
		if (value >= Inventory::Movement::MOVE_INTERNAL && value <= Inventory::Movement::MOVE_EQUIP)
			return static_cast<Movement>(value);

		std::cout << "Unknown Inventory::Movement value: [" << value << "]" << std::endl;

		return Inventory::Movement::MOVE_NONE;
	}
}