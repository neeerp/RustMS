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
#include <vector>

#include "UIState.h"

#include "Components/EquipTooltip.h"
#include "Components/ItemTooltip.h"
#include "Components/MapTooltip.h"
#include "Components/MacroTooltip.h"
#include "Components/SkillTooltip.h"
#include "Components/PlainTooltip.h"
#include "Components/TextTooltip.h"
#include "Components/MapTooltip.h"

#include "../Timer.h"

#include "../Template/EnumMap.h"
#include "../Template/Optional.h"
#include "../Character/CharStats.h"

namespace ms
{
	class UIStateGame : public UIState
	{
	public:
		UIStateGame();

		void draw(float inter, Point<int16_t> cursor) const override;

		void update() override;

		void doubleclick(Point<int16_t> pos) override;

		void rightclick(Point<int16_t> pos) override;

		void send_key(KeyType::Id type, int32_t action, bool pressed, bool escape) override;

		Cursor::State send_cursor(Cursor::State cursorstate, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset, Point<int16_t> pos) override;

		void send_close() override;

		void drag_icon(Icon *icon) override;

		void cancel_drag() override;

		void clear_tooltip(Tooltip::Parent parent) override;

		void show_equip(Tooltip::Parent parent, int16_t slot) override;

		void show_item(Tooltip::Parent parent, int32_t itemid, int64_t expiration, int16_t flags, bool owned,
					   int32_t remain_life) override;

		void show_skill(Tooltip::Parent parent, int32_t skill_id, int32_t level, int32_t masterlevel,
						int64_t expiration, bool levels) override;

		void show_text(Tooltip::Parent parent, std::string text) override;

		void show_macro(Tooltip::Parent parent, size_t index) override;

		void show_map(Tooltip::Parent parent, std::string name, std::string description) override;

		void show_zone(Tooltip::Parent parent, std::string title, std::string description) override;

		void show_line(Tooltip::Parent parent, std::string text) override;

		Iterator pre_add(UIElement::Type type, bool toggled, bool focused);

		void remove(UIElement::Type type) override;

		UIElement *get(UIElement::Type type) override;

		UIElement *get_front(std::list<UIElement::Type> types) override;

		UIElement *get_front(Point<int16_t> pos) override;

		bool modal_open() const override;

	private:
		const CharStats &stats;

		bool drop_icon(const Icon &icon, Point<int16_t> pos);

		void remove_icon();

		void remove_cursors();

		void remove_cursor(UIElement::Type type);

		// The window holding the focus. Once it closes, the focus goes back
		// to the one it opened over, such as a dialog under its notice.
		UIElement *get_focused();

		template<class T, typename...Args>
		void emplace(Args &&...args);

		EnumMap<UIElement::Type, UIElement::UPtr, UIElement::Type::NUM_TYPES> elements;
		std::list<UIElement::Type> elementorder;
		UIElement::Type focused;
		// The windows that held the focus before one opened over them, the
		// latest last.
		std::vector<UIElement::Type> focus_below;
		UIElement *dragged;

		EquipTooltip eqtooltip;
		ItemTooltip ittooltip;
		SkillTooltip sktooltip;
		TextTooltip tetooltip;
		MapTooltip matooltip;
		PlainTooltip pltooltip;
		MacroTooltip mctooltip;
		Optional<Tooltip> tooltip;
		Tooltip::Parent tooltipparent;

		Optional<Icon> draggedicon;

		std::map<Icon::IconType, UIElement::Type> icon_map = {
				{Icon::IconType::NONE,      UIElement::Type::NONE},
				{Icon::IconType::SKILL,     UIElement::Type::SKILLBOOK},
				{Icon::IconType::EQUIP,     UIElement::Type::EQUIPINVENTORY},
				{Icon::IconType::ITEM,      UIElement::Type::ITEMINVENTORY},
				{Icon::IconType::KEY,       UIElement::Type::KEYCONFIG},
				{Icon::IconType::MACRO_SKILL, UIElement::Type::SKILLMACRO},
				{Icon::IconType::MACRO,     UIElement::Type::SKILLMACRO},
				{Icon::IconType::NUM_TYPES, UIElement::Type::NUM_TYPES}
		};

		int16_t VWIDTH;
		int16_t VHEIGHT;
	};
}