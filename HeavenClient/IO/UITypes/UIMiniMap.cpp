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

#include "../Gameplay/MapleMap/Npc.h"

#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Laid out from footage, against the window's top-left corner or,
		// for the buttons, its right edge.
		const Point<int16_t> TITLE = { 8, 10 };
		const Point<int16_t> TITLE_MIN = { 8, 7 };
		const Point<int16_t> MARK = { 6, 23 };
		constexpr int16_t NAME_X = 49, STREET_Y = 22, NAME_Y = 38, NAME_RIGHT = 10;
		constexpr int16_t BT_Y = 6, BT_Y_MIN = 4, BT_MIN_RIGHT = 68, BT_MAX_RIGHT = 55, BT_MAP_RIGHT = 41;
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
			title.draw(position + TITLE_MIN);
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

			Texture portal_marker = markers["portal"];

			for (const auto& portal : portals)
				draw_marker(portal_marker, portal.second);

			Texture npc_marker = markers["npc"];

			for (const auto& npc : *Stage::get().get_npcs().get_npcs())
				draw_marker(npc_marker, npc.second->get_position());

			Texture other_marker = markers["another"];

			for (const auto& chr : *Stage::get().get_chars().get_chars())
				draw_marker(other_marker, chr.second->get_position());

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
		map = NxHelper::Map::get_map_node_name(mapid);

		nl::node minimap = map["miniMap"];
		has_map = minimap.size() > 0;
		canvas = minimap["canvas"];
		scale = static_cast<int16_t>(1 << static_cast<int>(minimap["mag"]));
		center_offset = Point<int16_t>(minimap["centerX"], minimap["centerY"]);
		mark = nl::nx::map["MapHelper.img"]["mark"][map["info"]["mapMark"].get_string()];

		NxHelper::Map::MapInfo info = NxHelper::Map::get_map_info_by_id(mapid);
		street.change_text(info.street_name);
		name.change_text(info.name);

		portals.clear();

		for (nl::node portal : map["portal"])
			if (static_cast<int>(portal["pt"]) == 2)
				portals.emplace_back(portal.name(), Point<int16_t>(portal["x"], portal["y"]));

		// Towns open with their mark and names.
		nl::node town = map["info"]["town"];
		set_type(!has_map ? Type::MIN : town && town.get_bool() ? Type::MAX : user_type);
	}

	void UIMiniMap::set_type(int8_t new_type)
	{
		type = new_type;

		if (type == Type::MIN)
		{
			width = MIN_WIDTH;
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
		buttons[BT_MIN]->set_position(Point<int16_t>(width - BT_MIN_RIGHT, y));
		buttons[BT_MAX]->set_position(Point<int16_t>(width - BT_MAX_RIGHT, y));
		buttons[BT_MAP]->set_position(Point<int16_t>(width - BT_MAP_RIGHT, y));
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

		Point<int16_t> portal_size = Texture(markers["portal"]).get_dimensions();

		for (const auto& [portal, at] : portals)
		{
			if (!covers(at, portal_size))
				continue;

			nl::node target = map["portal"][portal]["tm"];
			std::string category = NxHelper::Map::get_map_category(target);
			nl::node target_name = nl::nx::string["Map.img"][category][target]["mapName"];

			if (target_name)
			{
				UI::get().show_map(Tooltip::Parent::MINIMAP, target_name, "");

				return dstate;
			}
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
				UI::get().emplace<UIWorldMap>();
				break;
		}

		return buttons[buttonid]->get_state();
	}

	UIElement::Type UIMiniMap::get_type() const
	{
		return TYPE;
	}
}
