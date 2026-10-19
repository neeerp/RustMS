#include "MedalTag.h"

#include "../Data/EquipData.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The tag's text is drawn this far above the top of its cell, as
		// with the name tag box (GraphicsGL's NAMETAG background).
		constexpr int16_t TEXT_CELL = 6;

		MedalTagLayout::Piece piece(const Texture &texture)
		{
			Point<int16_t> origin = texture.get_origin();

			return { texture.width(), texture.height(), origin.x(), origin.y() };
		}
	}

	MedalTag::MedalTag(int32_t id) : itemid(id), shown(false), art(false), color(Color::Code::CWHITE)
	{
		if (!itemid)
			return;

		const EquipData &equip = EquipData::get(itemid);
		std::string name = MedalTagLayout::text(equip.get_itemdata().get_name());

		if (name.empty())
			return;

		shown = true;

		nl::node style = NxFiles::ui_classic["NameTag.img"]["medal"][std::to_string(equip.get_medaltag())];

		// Without art for its style the tag is a plain name tag (0x5f0e1d).
		if (!style)
		{
			text = Text(Text::Font::A13M, Text::Alignment::CENTER, Color::Name::WHITE, Text::Background::NAMETAG, name);
			return;
		}

		art = true;
		left = style["w"];
		tile = style["c"];
		right = style["e"];

		// clr is ARGB; v83 draws the name in Dotum 12, which this client
		// doesn't have.
		auto clr = static_cast<uint32_t>(style["clr"].get_integer(0xFFFFFFFF));
		color = Color(
			static_cast<uint8_t>(clr >> 16),
			static_cast<uint8_t>(clr >> 8),
			static_cast<uint8_t>(clr),
			static_cast<uint8_t>(clr >> 24)
		);
		text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, name, 0, false);
		layout = MedalTagLayout::layout(piece(left), piece(tile), piece(right), text.width());
	}

	MedalTag::MedalTag() : MedalTag(0) {}

	int32_t MedalTag::get_itemid() const
	{
		return itemid;
	}

	void MedalTag::draw(Point<int16_t> pos) const
	{
		if (!shown)
			return;

		if (!art)
		{
			text.draw(pos - Point<int16_t>(0, TEXT_CELL));
			return;
		}

		Point<int16_t> corner = pos - Point<int16_t>(layout.origin_x, 0);

		// Textures are drawn by their origins; the layout places their
		// corners.
		for (int16_t x : layout.tiles)
			tile.draw(corner + Point<int16_t>(x, layout.tile_y) + tile.get_origin());

		left.draw(corner + Point<int16_t>(layout.left.x, layout.left.y) + left.get_origin());
		right.draw(corner + Point<int16_t>(layout.right.x, layout.right.y) + right.get_origin());
		text.draw(DrawArgument(corner + Point<int16_t>(layout.text.x, layout.text.y - TEXT_CELL), color));
	}
}
