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
#include "UIMiniMap.h"

#include "UIWorldMap.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../Gameplay/MapleMap/FieldLimit.h"
#include "../Gameplay/MapleMap/MapTooltipRules.h"
#include "../Gameplay/MapleMap/Npc.h"

#include "../../Configuration.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Laid out from footage, against the window's top-left corner or,
		// for the buttons, its right edge.
		const Point<int16_t> TITLE = { 8, 10 };
		const Point<int16_t> MARK = { 6, 23 };
		constexpr int16_t NAME_X = 49, STREET_Y = 22, NAME_Y = 38, NAME_RIGHT = 10;
		constexpr int16_t BT_Y = 6, BT_Y_MIN = 4, BT_MIN_RIGHT = 68, BT_MAX_RIGHT = 55, BT_MAP_RIGHT = 41;
		// Folded, v83 (0x856ad1) writes the map name at (6, 3) and the street
		// 7 px after it, spaced by the bold names' widths +5 each. The bar is
		// those two spans +80 wide (0x85640e), its buttons 2 px further in (0x8583ea).
		constexpr int16_t MIN_TEXT_X = 6, MIN_TEXT_Y = 3 - 6, MIN_GAP = 7, MIN_EXTRA = 80, BT_MIN_SHIFT = 2;
	}

	UIMiniMap::Frame::Frame(nl::node src) :
		nw(src["nw"]), n(src["n"]), ne(src["ne"]), w(src["w"]), c(src["c"]), e(src["e"]),
		sw(src["sw"]), s(src["s"]), se(src["se"]) {}

	void UIMiniMap::Frame::draw(Point<int16_t> position, int16_t width, int16_t middle) const
	{
		int16_t inner = width - 2 * BORDER;
		int16_t bottom_y = top() + middle;

		nw.draw(position);
		n.draw(DrawArgument(position + Point<int16_t>(BORDER, 0), Point<int16_t>(inner, 0)));
		ne.draw(position + Point<int16_t>(width - BORDER, 0));

		if (middle > 0)
		{
			w.draw(DrawArgument(position + Point<int16_t>(0, top()), Point<int16_t>(0, middle)));
			backdrop.draw(DrawArgument(position + Point<int16_t>(BORDER, top()), Point<int16_t>(inner, middle)));
			c.draw(DrawArgument(position + Point<int16_t>(BORDER, top()), Point<int16_t>(inner, middle)));
			e.draw(DrawArgument(position + Point<int16_t>(width - BORDER, top()), Point<int16_t>(0, middle)));
		}

		sw.draw(position + Point<int16_t>(0, bottom_y));
		s.draw(DrawArgument(position + Point<int16_t>(BORDER, bottom_y), Point<int16_t>(inner, 0)));
		se.draw(position + Point<int16_t>(width - BORDER, bottom_y));
	}

	int16_t UIMiniMap::Frame::top() const
	{
		return n.get_dimensions().y();
	}

	int16_t UIMiniMap::Frame::bottom() const
	{
		return s.get_dimensions().y();
	}

	UIMiniMap::UIMiniMap(const CharStats &stats) : UIDragElement<PosMINIMAP>(Point<int16_t>(MIN_WIDTH, 20)), stats(stats)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["MiniMap"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		normal_frame = Frame(src["MinMap"]);
		max_frame = Frame(src["MaxMap"]);
		min_left = src["Min"]["w"];
		min_center = src["Min"]["c"];
		min_right = src["Min"]["e"];
		title = src["title"];
		markers = nl::nx::map["MapHelper.img"]["minimap"];

		buttons[BT_MIN] = std::make_unique<MapleButton>(basic["BtMin"]);
		buttons[BT_MAX] = std::make_unique<MapleButton>(basic["BtMax"]);
		buttons[BT_MAP] = std::make_unique<MapleButton>(src["BtMap"]);

		street = OutlinedText(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::WHITE, Color::Name::SANJUAN);
		name = OutlinedText(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::WHITE, Color::Name::SANJUAN);
		min_street = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		min_name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		user_type = Setting<MiniMapType>::get().load();
		type = user_type;
	}

	void UIMiniMap::draw(float alpha) const
	{
		if (type == Type::MIN)
		{
			min_left.draw(position);
			min_center.draw(DrawArgument(position + Point<int16_t>(min_left.get_dimensions().x(), 0),
				Point<int16_t>(width - min_left.get_dimensions().x() - min_right.get_dimensions().x(), 0)));
			min_right.draw(position + Point<int16_t>(width - min_right.get_dimensions().x(), 0));
			min_name.draw(position + Point<int16_t>(MIN_TEXT_X, MIN_TEXT_Y));
			min_street.draw(position + Point<int16_t>(name.width() + 5 + MIN_GAP, MIN_TEXT_Y));
		}
		else
		{
			const Frame& frame = type == Type::MAX ? max_frame : normal_frame;
			frame.draw(position, width, canvas.get_dimensions().y());
			title.draw(position + TITLE);
			canvas.draw(position + to_window(-center_offset));

			if (type == Type::MAX)
			{
				mark.draw(position + MARK);
				street.draw(position + Point<int16_t>(NAME_X, STREET_Y));
				name.draw(position + Point<int16_t>(NAME_X, NAME_Y));
			}

			// Markers stand on their spot, centred.
			auto draw_marker = [&](const Texture& marker, Point<int16_t> map_position)
			{
				Point<int16_t> size = marker.get_dimensions();
				marker.draw(position + to_window(map_position) - Point<int16_t>(size.x() / 2, size.y()));
			};

			// v83's order (0x85a126): others, the partner, NPCs, portals and
			// the player on top. The partner's dot replaces their other one.
			Texture other_marker = markers["another"];
			const PartnerSpot& partner = Stage::get().get_player().get_partner();
			const MapObject* partner_char = nullptr;

			for (const auto& chr : *Stage::get().get_chars().get_chars())
			{
				if (partner.is(chr.first))
					partner_char = chr.second.get();
				else
					draw_marker(other_marker, chr.second->get_position());
			}

			if (partner_char)
				draw_marker(markers["match"], partner_char->get_position());

			Texture npc_marker = markers["npc"];

			for (const auto& npc : *Stage::get().get_npcs().get_npcs())
				draw_marker(npc_marker, npc.second->get_position());

			Texture portal_marker = markers["portal"];

			for (const auto& portal : portals)
				draw_marker(portal_marker, portal.position);

			draw_marker(markers["user"], Stage::get().get_player().get_position());
		}

		UIElement::draw(alpha);
	}

	void UIMiniMap::update()
	{
		if (Stage::get().get_mapid() != mapid)
			change_map();

		UIElement::update();
	}

	void UIMiniMap::change_map()
	{
		mapid = Stage::get().get_mapid();
		map = NxHelper::Map::get_map_body(mapid);

		nl::node minimap = map["miniMap"];
		nl::node map_info = NxHelper::Map::get_map_node_name(mapid)["info"];
		has_map = field_limit::shows_minimap(Configuration::get().get_admin(), minimap.size() > 0,
											 map_info["hideMinimap"].get_bool(), map_info["miniMapOnOff"].get_bool(),
											 switched_on);
		canvas = minimap["canvas"];
		scale = static_cast<int16_t>(1 << static_cast<int>(minimap["mag"]));
		center_offset = Point<int16_t>(minimap["centerX"], minimap["centerY"]);
		mark = nl::nx::map["MapHelper.img"]["mark"][map_info["mapMark"].get_string()];

		NxHelper::Map::MapInfo info = NxHelper::Map::get_map_info_by_id(mapid);
		street.change_text(info.street_name);
		name.change_text(info.name);
		min_street.change_text(info.street_name);
		min_name.change_text(info.name);

		portals.clear();

		for (nl::node portal : map["portal"])
		{
			int32_t type = portal["pt"];

			if (!map_tooltips::minimap_shows(type))
				continue;

			int32_t target = portal["tm"];
			bool tip = map_tooltips::minimap_tip(type, target, portal["hideTooltip"].get_integer() != 0);
			portals.push_back({ Point<int16_t>(portal["x"], portal["y"]), tip, portal["pn"], target });
		}

		// Towns open with their mark and names.
		nl::node town = map_info["town"];
		set_type(!has_map ? Type::MIN : town && town.get_bool() ? Type::MAX : user_type);
	}

	bool UIMiniMap::switched_on = true;

	void UIMiniMap::switch_on(bool on)
	{
		switched_on = on;

		if (auto minimap = UI::get().get_element<UIMiniMap>())
			minimap->change_map();
	}

	void UIMiniMap::set_type(int8_t new_type)
	{
		type = new_type;

		if (type == Type::MIN)
		{
			width = street.width() + 5 + name.width() + 5 + MIN_EXTRA;
			dimension = Point<int16_t>(width, min_center.get_dimensions().y());
		}
		else
		{
			width = std::max<int16_t>(MIN_WIDTH, canvas.get_dimensions().x() + 2 * BORDER);

			if (type == Type::MAX)
				width = std::max<int16_t>(width, NAME_X + std::max(street.width(), name.width()) + NAME_RIGHT);

			const Frame& frame = type == Type::MAX ? max_frame : normal_frame;
			dimension = Point<int16_t>(width, frame.top() + canvas.get_dimensions().y() + frame.bottom());
		}

		dragarea = Point<int16_t>(width, 20);

		int16_t y = type == Type::MIN ? BT_Y_MIN : BT_Y;
		int16_t in = type == Type::MIN ? BT_MIN_SHIFT : 0;
		buttons[BT_MIN]->set_position(Point<int16_t>(width - BT_MIN_RIGHT - in, y));
		buttons[BT_MAX]->set_position(Point<int16_t>(width - BT_MAX_RIGHT - in, y));
		buttons[BT_MAP]->set_position(Point<int16_t>(width - BT_MAP_RIGHT - in, y));
		buttons[BT_MIN]->set_state(type == Type::MIN ? Button::State::DISABLED : Button::State::NORMAL);
		buttons[BT_MAX]->set_state(type == Type::MAX || !has_map ? Button::State::DISABLED : Button::State::NORMAL);
	}

	void UIMiniMap::choose_type(int8_t new_type)
	{
		user_type = new_type;
		Setting<MiniMapType>::get().save(user_type);
		set_type(user_type);
	}

	Point<int16_t> UIMiniMap::to_window(Point<int16_t> map_position) const
	{
		Point<int16_t> size = canvas.get_dimensions();
		Point<int16_t> corner((width - size.x()) / 2, top());

		return corner + (map_position + center_offset) / scale;
	}

	int16_t UIMiniMap::top() const
	{
		return (type == Type::MAX ? max_frame : normal_frame).top();
	}

	void UIMiniMap::remove_cursor()
	{
		UIDragElement::remove_cursor();

		UI::get().clear_tooltip(Tooltip::Parent::MINIMAP);
	}

	Cursor::State UIMiniMap::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged || type == Type::MIN)
			return dstate;

		Point<int16_t> cursor = cursorpos - position;

		// The spot a marker of the given size covers.
		auto covers = [&](Point<int16_t> map_position, Point<int16_t> size)
		{
			Point<int16_t> foot = to_window(map_position);
			return Rectangle<int16_t>(foot - Point<int16_t>(size.x() / 2, size.y()), foot + Point<int16_t>(size.x() - size.x() / 2, 0))
				.contains(cursor);
		};

		Point<int16_t> npc_size = Texture(markers["npc"]).get_dimensions();

		for (const auto& npc : *Stage::get().get_npcs().get_npcs())
		{
			if (covers(npc.second->get_position(), npc_size))
			{
				auto n = static_cast<Npc *>(npc.second.get());
				UI::get().show_map(Tooltip::Parent::MINIMAP, n->get_name(), n->get_func());

				return dstate;
			}
		}

		// v83 tries the portals last to first, each in a small spot around
		// its marker (FUN_0085ade2). A tip is the String
		// ToolTipHelp.img/PortalTooltip text for the portal, or else where it
		// leads; with neither, the last tip stays up.
		for (auto portal = portals.rbegin(); portal != portals.rend(); ++portal)
		{
			if (!portal->tip)
				continue;

			Point<int16_t> at = to_window(portal->position);

			if (!map_tooltips::contains(map_tooltips::minimap_portal_spot(at.x(), at.y()), cursor.x(), cursor.y()))
				continue;

			nl::node tip = nl::nx::string["ToolTipHelp.img"]["PortalTooltip"][std::to_string(mapid)][portal->name];
			std::string text = tip.data_type() == nl::node::type::string ? tip.get_string() : "";

			if (text.empty())
				text = NxHelper::Map::get_map_info_by_id(portal->target).name;

			if (!text.empty())
				UI::get().show_line(Tooltip::Parent::MINIMAP, text);

			return dstate;
		}

		UI::get().clear_tooltip(Tooltip::Parent::MINIMAP);

		return dstate;
	}

	void UIMiniMap::send_key(int32_t, bool, bool)
	{
		if (!has_map)
			return;

		choose_type(type == Type::MAX ? Type::MIN : type + 1);
	}

	Button::State UIMiniMap::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case BT_MIN:
				choose_type(type - 1);
				break;
			case BT_MAX:
				choose_type(type + 1);
				break;
			case BT_MAP:
				UIWorldMap::open();
				// Back to normal, or the button stays pressed and takes no
				// more clicks.
				return Button::State::NORMAL;
		}

		return buttons[buttonid]->get_state();
	}

	UIElement::Type UIMiniMap::get_type() const
	{
		return TYPE;
	}

	int16_t UIMiniMap::get_width() const
	{
		return dimension.x();
	}
}
