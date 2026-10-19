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

#include <cstdint>
#include <map>

#include "../UIElement.h"

#include "../Components/Charset.h"

#include "../../Character/CharStats.h"
#include "../../Graphics/Animation.h"
#include "../../Graphics/Text.h"

namespace ms
{
	// The v83 status bar along the bottom of the screen: level, job and name,
	// the HP, MP and EXP gauges, the menu buttons, the quick slot panel and the
	// two popup menus. The chat strip in its top-left is UIChatBar's.
	class UIStatusBar : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::STATUSBAR;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		// The bar is 71 px tall; layouts are in pixels from its top-left.
		static constexpr int16_t HEIGHT = 71;

		// The quick-slot panel's top, from the bar's top.
		static constexpr int16_t QUICKSLOT_TOP = -76;

		// The chat strip, which UIChatBar draws and handles.
		static constexpr Rectangle<int16_t> CHAT_STRIP = Rectangle<int16_t>(0, 570, 7, 29);

		UIStatusBar(const CharStats &stats);

		void draw(float alpha) const override;

		void update() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;
		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void doubleclick(Point<int16_t> cursorpos) override;
		bool send_icon(const Icon &icon, Point<int16_t> cursorpos) override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// Show or hide the quick slot panel.
		void toggle_qs();

		// Open or close the shortcut menu.
		void toggle_menu();

		void remove_menus();

		bool is_menu_active();

		// The equip, item, stat and skill keys, left to right.
		enum Key : uint8_t
		{
			KEY_EQUIP,
			KEY_ITEM,
			KEY_STAT,
			KEY_SKILL,
			NUM_KEYS
		};

		// Makes a key glow (StatusBar.img/<key>/ani, FUN_008dfe21) until it is clicked, or for 90 seconds.
		// A key that already glows keeps its time.
		void show_key_hint(Key key);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;
		bool button_covered(uint16_t buttonid, Point<int16_t> cursorpos) const override;

	private:
		enum Menu
		{
			NONE,
			GAME,
			SHORTCUT
		};

		float getexppercent() const;

		float gethppercent() const;

		float getmppercent() const;

		// A gauge's warning flash. v83 (FUN_008d850b) plays it once each
		// time the value falls while under the warning level.
		struct GaugeFlash
		{
			Animation animation;
			Point<int16_t> pos;
			bool active = false;
			// The value at the last check; -1 until the first.
			int32_t last = -1;

			void check(int32_t value, int32_t max);
			void update();
			void draw(Point<int16_t> parent) const;
		};

		void toggle_menu(Menu menu);

		void set_menu(Menu menu);

		void draw_gauge(int16_t left, int16_t width, float percent) const;

		void draw_number(const std::string &text, Point<int16_t> pos) const;

		void draw_quickslots() const;
		int16_t quickslot_at(Point<int16_t> cursorpos) const;

		Rectangle<int16_t> get_menu_bounds() const;

		enum Buttons : uint16_t
		{
			BT_CLAIM,
			BT_EQUIP,
			BT_INVEN,
			BT_STAT,
			BT_SKILL,
			BT_KEYSET,
			BT_QUICKSLOT_SHOW,
			BT_QUICKSLOT_HIDE,
			BT_CASHSHOP,
			BT_TRADE,
			BT_MENU,
			BT_SHORTCUT,
			BT_GAME_CHANNEL,
			BT_GAME_OPTION,
			BT_GAME_SYSTEM,
			BT_GAME_QUIT,
			BT_SHORTCUT_EQUIP,
			BT_SHORTCUT_ITEM,
			BT_SHORTCUT_STAT,
			BT_SHORTCUT_SKILL,
			BT_SHORTCUT_QUEST,
			BT_SHORTCUT_COMMUNITY,
			BT_SHORTCUT_MESSENGER,
			BT_SHORTCUT_MONSTERBOOK
		};

		const CharStats &stats;

		Texture backgrnd;
		Texture backgrnd2;
		Texture gaugebar;
		Texture gaugegray;
		Texture graduation;
		GaugeFlash hpflash;
		GaugeFlash mpflash;
		Texture quickslot;
		Texture keylabels[8];
		Texture gamemenu;
		Texture shortcutmenu;
		std::map<int32_t, Texture> action_icons;
		Charset number;
		Charset levelnumber;
		Charset itemnumber;
		Text joblabel;
		Text namelabel;

		int16_t VWIDTH;
		int16_t VHEIGHT;

		// The menu buttons and quick slot panel keep to the right edge.
		int16_t right;
		Point<int16_t> quickslot_pos;
		Point<int16_t> gamemenu_pos;
		Point<int16_t> shortcutmenu_pos;

		bool quickslot_active;
		Menu menu;

		Animation key_hints[Key::NUM_KEYS];
		// Steps left for each key's glow; 0 when it is off.
		uint16_t key_hint_left[Key::NUM_KEYS] = {};
	};
}
