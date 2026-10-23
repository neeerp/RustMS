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
#include "MapMerchants.h"

#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

#include <cmath>
#include <cstdio>
#include <utility>

namespace ms
{
	namespace
	{
		// The permit's art (SP 0xd7c, 0xd7e, 0xd85).
		nl::node permit(int32_t itemid)
		{
			return nl::nx::item["Cash"]["0503.img"][string_format::extend_id(itemid, 8)];
		}

		// A skin piece's top left: centred on the employee, its foot 2 below
		// the employee's (0x48df5a, 0x48e85a). v83 ignores the canvas origin.
		Point<int16_t> piece_at(const Texture &piece, Point<int16_t> feet)
		{
			Point<int16_t> size = piece.get_dimensions();

			return feet + Point<int16_t>(-size.x() / 2, 2 - size.y());
		}

		void draw_piece(const Texture &piece, Point<int16_t> feet)
		{
			piece.draw(piece_at(piece, feet) + piece.get_origin());
		}
	}

	void MapMerchants::spawn(int32_t owner, int32_t itemid, Point<int16_t> position, int16_t fh, std::string name,
							 const std::optional<MiniRoomBox> &box)
	{
		// One already standing stays as it was (0x510ebb).
		if (merchants.count(owner))
			return;

		char tag[64];
		std::snprintf(tag, sizeof(tag), "%s's Hired Merchant", name.c_str());

		Merchant merchant{ itemid, position, fh, std::nullopt, Animation(permit(itemid)["employee"]["stand"]),
						   Text(Text::Font::A13M, Text::Alignment::CENTER, Color::Name::WHITE, Text::Background::NAMETAG,
								tag) };
		skin(merchant, box);
		merchants.emplace(owner, std::move(merchant));
	}

	void MapMerchants::change(int32_t owner, const std::optional<MiniRoomBox> &box)
	{
		auto iter = merchants.find(owner);

		if (iter != merchants.end())
			skin(iter->second, box);
	}

	void MapMerchants::skin(Merchant &merchant, const std::optional<MiniRoomBox> &box)
	{
		merchant.balloon.change(box);
		merchant.skinned = box.has_value();

		if (!box)
			return;

		// The skin is the room's, `050300%02d` (SP 0xd80).
		char id[16];
		std::snprintf(id, sizeof(id), "050300%02d", box->skin);
		nl::node art = nl::nx::item["Cash"]["0503.img"][std::string(id)];

		if (!art["skin"]["signboard"])
			art = permit(merchant.itemid);

		merchant.backgrnd = art["skin"]["backgrnd"];
		merchant.shop = art["skin"]["shop"];
		merchant.effect = Animation(art["effect"]);
	}

	void MapMerchants::remove(int32_t owner)
	{
		merchants.erase(owner);
	}

	bool MapMerchants::contains(int32_t owner) const
	{
		return merchants.count(owner) > 0;
	}

	void MapMerchants::update(const Physics &physics)
	{
		for (auto &iter : merchants)
		{
			Merchant &merchant = iter.second;

			// Footholds come with the map, after the merchants it starts with.
			// One with no known foothold takes the one under it.
			if (!merchant.layer)
			{
				const FootholdTree &fht = physics.get_fht();
				uint16_t fh = fht.get_fh(merchant.fh).id() ? merchant.fh : fht.get_fhid_below(merchant.position);
				merchant.layer = static_cast<Layer::Id>(fht.get_fh(fh).layer());
			}

			merchant.employee.update();

			if (merchant.skinned)
				merchant.effect.update();
		}
	}

	void MapMerchants::draw(Layer::Id layer, double viewx, double viewy, float alpha) const
	{
		Point<int16_t> view(static_cast<int16_t>(std::round(viewx)), static_cast<int16_t>(std::round(viewy)));

		for (const auto &iter : merchants)
		{
			const Merchant &merchant = iter.second;

			if (merchant.layer.value_or(Layer::ZERO) != layer)
				continue;

			Point<int16_t> feet = merchant.position + view;

			// Behind the employee, then in front of it (0x48d731, 0x48d6eb).
			if (merchant.skinned)
				draw_piece(merchant.backgrnd, feet);

			merchant.employee.draw(DrawArgument(feet), alpha);

			if (merchant.skinned)
			{
				draw_piece(merchant.shop, feet);

				// 05030008's sparkle, the size of the skin (0x48dc42).
				if (merchant.effect.get_duration() > 0)
					merchant.effect.draw(DrawArgument(piece_at(merchant.shop, feet) + merchant.effect.get_origin()),
										 alpha);
			}

			merchant.name.draw(feet + Point<int16_t>(0, -4));
		}
	}

	void MapMerchants::draw_balloons(double viewx, double viewy) const
	{
		Point<int16_t> view(static_cast<int16_t>(std::round(viewx)), static_cast<int16_t>(std::round(viewy)));

		for (const auto &iter : merchants)
			iter.second.balloon.draw(iter.second.position + view, 0);
	}

	void MapMerchants::clear()
	{
		merchants.clear();
	}

	std::optional<MapMerchants::Hit> MapMerchants::at(Point<int16_t> position) const
	{
		// The whole signboard canvas is the target (0x48e136).
		for (auto iter = merchants.rbegin(); iter != merchants.rend(); ++iter)
		{
			if (int32_t serial = iter->second.balloon.serial_at(iter->second.position, 0, position))
				return Hit{ iter->first, serial };
		}

		return std::nullopt;
	}
}
