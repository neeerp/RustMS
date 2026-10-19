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

#include "Keyboard.h"
#include "UIElement.h"

#include "Components/Tooltip.h"

#include "../Template/EnumMap.h"

#include <memory>
#include <list>

namespace ms
{
	class UIState
	{
	public:
		using Iterator = EnumMap<UIElement::Type, std::unique_ptr<UIElement>, UIElement::Type::NUM_TYPES>::iterator;

		virtual ~UIState()
		{}

		virtual void draw(float inter, Point<int16_t> cursor) const = 0;

		virtual void update() = 0;

		virtual void doubleclick(Point<int16_t> pos) = 0;

		virtual void rightclick(Point<int16_t> pos) = 0;

		virtual void send_key(KeyType::Id type, int32_t action, bool pressed, bool escape) = 0;

		virtual Cursor::State send_cursor(Cursor::State mst, Point<int16_t> pos) = 0;

		virtual void send_scroll(double yoffset) = 0;

		virtual void send_close() = 0;

		virtual void drag_icon(Icon *icon) = 0;

		// Drop any dragged icon without placing it, e.g. before the icons it
		// points into are rebuilt.
		virtual void cancel_drag() {}

		virtual void clear_tooltip(Tooltip::Parent parent) = 0;

		virtual void show_equip(Tooltip::Parent parent, int16_t slot) = 0;

		virtual void show_item(Tooltip::Parent parent, int32_t itemid, int64_t expiration, int16_t flags,
							   bool owned) = 0;

		// Without levels, the tooltip leaves out the levels' effects.
		virtual void show_skill(Tooltip::Parent parent, int32_t skill_id, int32_t level, int32_t masterlevel,
								int64_t expiration, bool levels) = 0;

		virtual void show_text(Tooltip::Parent parent, std::string text) = 0;

		virtual void
		show_map(Tooltip::Parent parent, std::string name, std::string description) = 0;

		// A map zone's title and description in v83's plain navy box.
		virtual void show_zone(Tooltip::Parent parent, std::string title, std::string description) {}

		// One line in v83's plain navy box, as for a portal on the minimap.
		virtual void show_line(Tooltip::Parent parent, std::string text) {}

		virtual Iterator pre_add(UIElement::Type type, bool toggled, bool focused) = 0;

		virtual void remove(UIElement::Type type) = 0;
		
		virtual UIElement* get(UIElement::Type type) = 0;

		virtual UIElement* get_front(std::list<UIElement::Type> types) = 0;

		virtual UIElement* get_front(Point<int16_t> pos) = 0;

		// Whether a window that holds the cursor, such as a notice or an
		// NPC's talk, is open.
		virtual bool modal_open() const { return false; }
	};

	class UIStateNull : public UIState
	{
		void draw(float, Point<int16_t>) const override {}
		void update() override {}
		void doubleclick(Point<int16_t>) override {}
		void rightclick(Point<int16_t>) override {}
		void send_key(KeyType::Id, int32_t, bool, bool) override {}
		Cursor::State send_cursor(Cursor::State, Point<int16_t>) override { return Cursor::State::IDLE; }
		void send_scroll(double yoffset) override {}
		void send_close() override {}
		void drag_icon(Icon*) override {}
		void clear_tooltip(Tooltip::Parent) override {}
		void show_equip(Tooltip::Parent, int16_t) override {}
		void show_item(Tooltip::Parent, int32_t, int64_t, int16_t, bool) override {}
		void show_skill(Tooltip::Parent, int32_t, int32_t, int32_t, int64_t, bool) override {}
		void show_text(Tooltip::Parent, std::string) override {}
		void show_map(Tooltip::Parent, std::string, std::string) override {}
		Iterator pre_add(UIElement::Type, bool, bool) override { return { nullptr, UIElement::Type::NUM_TYPES }; }
		void remove(UIElement::Type) override {}
		UIElement* get(UIElement::Type) override { return nullptr; }
		UIElement* get_front(std::list<UIElement::Type>) override { return nullptr; }
		UIElement* get_front(Point<int16_t>) override { return nullptr; }
	};
}