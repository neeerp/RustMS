#include "MedalTag.h"

#include "../Data/EquipData.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	MedalTag::MedalTag(int32_t id) : itemid(id)
	{
		if (!itemid)
			return;

		const EquipData &equip = EquipData::get(itemid);
		std::string name = MedalTagLayout::text(equip.get_itemdata().get_name());

		tag = StyledTag(NxFiles::ui_classic["NameTag.img"]["medal"][std::to_string(equip.get_medaltag())], name);
	}

	MedalTag::MedalTag() : MedalTag(0) {}

	int32_t MedalTag::get_itemid() const
	{
		return itemid;
	}

	void MedalTag::draw(Point<int16_t> pos) const
	{
		tag.draw(pos);
	}
}
