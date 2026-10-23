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
#include "UIWorldSelect.h"

#include "UILogin.h"
#include "UILoginNotice.h"
#include "UILoginWait.h"
#include "UINotice.h"

#include "../LoginScene.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Session.h"
#include "../../Net/Packets/LoginPackets.h"

namespace ms
{
	namespace
	{
		// Positions on screen with the camera at the world select stop,
		// measured against period footage.
		constexpr uint8_t PLANK_SLOTS = 15;
		constexpr int16_t PLANK_X = 232;
		constexpr int16_t PLANK_STEP = 27;
		constexpr int16_t PLANK_Y = 104;
		constexpr int16_t PLANK_WIDTH = 26;
		constexpr int16_t PLANK_HEIGHT = 95;

		const Point<int16_t> SCROLL_POS = Point<int16_t>(158, 79);
		const Point<int16_t> CHANNEL_BACKGROUND_POS = Point<int16_t>(420, 341);
		const Point<int16_t> WORLD_NAME_POS = Point<int16_t>(229, 220);

		// Channels fill four columns, up to twenty in all.
		constexpr uint8_t CHANNEL_SLOTS = 20;
		constexpr uint8_t CHANNEL_COLUMNS = 4;
		const Point<int16_t> CHANNEL_POS = Point<int16_t>(231, 277);
		const Point<int16_t> CHANNEL_STEP = Point<int16_t>(94, 32);
		constexpr int16_t CHANNEL_WIDTH = 92;
		constexpr int16_t CHANNEL_HEIGHT = 30;

		// The load gauge fills the slot under the channel's name, and a load
		// of 1000 fills it.
		const Point<int16_t> GAUGE_OFFSET = Point<int16_t>(44, 20);
		constexpr double GAUGE_FULL = 1000.0;

		// The red circle drawn around the selected channel's name.
		const Point<int16_t> CHANNEL_SELECT_OFFSET = Point<int16_t>(18, 9);

		// The bottom centre of the balloon over the rope bar.
		const Point<int16_t> BALLOON_POS = Point<int16_t>(407, 82);

		Point<int16_t> plank_pos(size_t slot)
		{
			return Point<int16_t>(PLANK_X + PLANK_STEP * static_cast<int16_t>(slot), PLANK_Y);
		}

		Point<int16_t> channel_pos(size_t channel)
		{
			auto column = static_cast<int16_t>(channel % CHANNEL_COLUMNS);
			auto row = static_cast<int16_t>(channel / CHANNEL_COLUMNS);

			return CHANNEL_POS + Point<int16_t>(CHANNEL_STEP.x() * column, CHANNEL_STEP.y() * row);
		}
	}

	UIWorldSelect::UIWorldSelect() : UIElement(Point<int16_t>(0, 0), Point<int16_t>(800, 600))
	{
		LoginScene::get().move_to(LoginScene::Step::WORLD_SELECT);

		nl::node login = NxFiles::ui_classic["Login.img"];
		nl::node worldselect = login["WorldSelect"];
		nl::node channel = worldselect["channel"];

		empty_plank = worldselect["BtWorld"]["e"]["normal"]["0"];

		scroll_open = worldselect["scroll"]["0"];
		scroll_close = worldselect["scroll"]["1"];
		scroll_opened = worldselect["scroll"]["0"]["3"];
		channel_background = worldselect["chBackgrn"];
		gauge = channel["chgauge"];
		channel_select = channel["chSelect"];
		channel_selected = channel["chSelect"]["3"];

		for (uint8_t i = 0; i < CHANNEL_SLOTS; i++)
			channels.emplace_back(channel[std::to_string(i)]["normal"]);

		buttons[Buttons::BT_ENTERWORLD] = std::make_unique<MapleButton>(worldselect["BtGoworld"], Point<int16_t>(515, 436));
		buttons[Buttons::BT_VIEWRECOMMENDED] = std::make_unique<MapleButton>(worldselect["BtViewChoice"], Point<int16_t>(0, 125));
		buttons[Buttons::BT_VIEWALLCHAR] = std::make_unique<MapleButton>(login["ViewAllChar"]["BtVAC"], Point<int16_t>(0, 381));
		buttons[Buttons::BT_BACK] = std::make_unique<MapleButton>(login["Common"]["BtStart"], Point<int16_t>(0, 433));

		buttons[Buttons::BT_ENTERWORLD]->set_active(false);

		// The server recommends no worlds and can't list every character.
		buttons[Buttons::BT_VIEWRECOMMENDED]->set_state(Button::State::DISABLED);
		buttons[Buttons::BT_VIEWALLCHAR]->set_state(Button::State::DISABLED);

		chatballoon.change_text("Please select the World you would like to play in.");

		channelid = Setting<DefaultChannel>::get().load();

		if (Configuration::get().get_auto_login())
		{
			auto world = Configuration::get().get_auto_world();
			auto channel = Configuration::get().get_auto_channel();

			Configuration::get().set_worldid(world);
			Configuration::get().set_channelid(channel);

			UI::get().emplace<UILoginWait>();
			auto loginwait = UI::get().get_element<UILoginWait>();

			if (loginwait && loginwait->is_active())
				CharlistRequestPacket(world, channel).dispatch();
		}
	}

	void UIWorldSelect::draw(float alpha) const
	{
		switch (scroll)
		{
		case Scroll::OPENING:
			scroll_open.draw(position + SCROLL_POS, alpha);
			break;
		case Scroll::CLOSING:
			scroll_close.draw(position + SCROLL_POS, alpha);
			break;
		case Scroll::OPEN:
		{
			scroll_opened.draw(position + SCROLL_POS);
			channel_background.draw(position + CHANNEL_BACKGROUND_POS);
			world_name.draw(position + WORLD_NAME_POS);

			const World &world = worlds[selected];

			for (uint8_t i = 0; i < world.channelcount && i < CHANNEL_SLOTS; i++)
			{
				Point<int16_t> pos = position + channel_pos(i);
				channels[i].draw(pos);

				int32_t load = i < world.chloads.size() ? world.chloads[i] : 0;
				double fill = std::clamp(load / GAUGE_FULL, 0.0, 1.0);
				auto width = static_cast<int16_t>(gauge.width() * fill);

				if (width > 0)
					gauge.draw(pos + GAUGE_OFFSET, Rectangle<int16_t>(0, width, 0, gauge.height()));
			}

			Point<int16_t> circle = position + channel_pos(channelid) + CHANNEL_SELECT_OFFSET;

			if (channel_select_done)
				channel_selected.draw(circle);
			else
				channel_select.draw(circle, alpha);

			break;
		}
		default:
			break;
		}

		int16_t pressed = status_pending ? requested : selected;

		for (size_t i = 0; i < PLANK_SLOTS; i++)
		{
			Point<int16_t> pos = position + plank_pos(i);

			if (i >= planks.size())
			{
				empty_plank.draw(pos);
				continue;
			}

			const Plank &plank = planks[i];

			if (static_cast<int16_t>(i) == pressed)
				plank.pressed.draw(pos);
			else if (plank.hovered && plank.hover_done)
				plank.mouseover_end.draw(pos);
			else if (plank.hovered)
				plank.mouseover.draw(pos, alpha);
			else
				plank.normal.draw(pos);

			if (static_cast<int16_t>(i) == focused)
				plank.focused.draw(pos);
		}

		UIElement::draw_buttons(alpha);

		chatballoon.draw(position + BALLOON_POS);
	}

	void UIWorldSelect::update()
	{
		UIElement::update();

		for (auto &plank : planks)
			if (plank.hovered && !plank.hover_done && plank.mouseover.update())
				plank.hover_done = true;

		if (scroll == Scroll::OPENING && scroll_open.update())
		{
			scroll = Scroll::OPEN;
			buttons[Buttons::BT_ENTERWORLD]->set_active(true);
		} else if (scroll == Scroll::CLOSING && scroll_close.update())
		{
			scroll = Scroll::CLOSED;
		} else if (scroll == Scroll::OPEN && !channel_select_done && channel_select.update())
		{
			channel_select_done = true;
		}
	}

	Cursor::State UIWorldSelect::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State ret = Cursor::State::IDLE;

		int16_t hovered = plank_at(cursorpos);

		for (size_t i = 0; i < planks.size(); i++)
		{
			Plank &plank = planks[i];
			bool over = static_cast<int16_t>(i) == hovered;

			if (over && !plank.hovered)
			{
				Sound(Sound::Name::BUTTONOVER).play();

				plank.mouseover.reset();
				plank.hover_done = false;
			}

			plank.hovered = over;
		}

		if (hovered >= 0)
		{
			if (clicked)
			{
				select_world(static_cast<uint8_t>(hovered));

				return Cursor::State::IDLE;
			}

			return Cursor::State::CANCLICK;
		}

		int16_t channel = channel_at(cursorpos);

		if (channel >= 0)
		{
			if (clicked)
			{
				if (channel == channelid)
				{
					enter_world();
				} else
				{
					Sound(Sound::Name::BUTTONCLICK).play();
					select_channel(static_cast<uint8_t>(channel));
				}

				return Cursor::State::IDLE;
			}

			return Cursor::State::CANCLICK;
		}

		for (auto &btit : buttons)
		{
			Button *button = btit.second.get();

			if (button->is_active() && button->bounds(position).contains(cursorpos))
			{
				Button::State state = button->get_state();

				if (state == Button::State::DISABLED)
					continue;

				if (state == Button::State::NORMAL)
				{
					Sound(Sound::Name::BUTTONOVER).play();
					button->set_state(Button::State::MOUSEOVER);
				}

				if (clicked)
				{
					Sound(Sound::Name::BUTTONCLICK).play();
					button->set_state(button_pressed(btit.first));
				} else
				{
					ret = Cursor::State::CANCLICK;
				}
			} else if (button->get_state() == Button::State::MOUSEOVER)
			{
				button->set_state(Button::State::NORMAL);
			}
		}

		return ret;
	}

	void UIWorldSelect::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		auto quitconfirm = UI::get().get_element<UIQuitConfirm>();

		if (quitconfirm && quitconfirm->is_active())
			return UI::get().send_key(keycode, pressed);

		if (scroll == Scroll::OPEN)
		{
			const World &world = worlds[selected];
			uint8_t count = std::min<uint8_t>(world.channelcount, CHANNEL_SLOTS);

			if (count == 0)
				return;

			int16_t next = channelid;

			if (keycode == KeyAction::Id::LEFT)
				next = (channelid + count - 1) % count;
			else if (keycode == KeyAction::Id::RIGHT || keycode == KeyAction::Id::TAB)
				next = (channelid + 1) % count;
			else if (keycode == KeyAction::Id::UP && channelid >= CHANNEL_COLUMNS)
				next = channelid - CHANNEL_COLUMNS;
			else if (keycode == KeyAction::Id::DOWN && channelid + CHANNEL_COLUMNS < count)
				next = channelid + CHANNEL_COLUMNS;
			else if (keycode == KeyAction::Id::RETURN)
				return enter_world();
			else if (escape)
				return close_scroll();

			if (next != channelid)
				select_channel(static_cast<uint8_t>(next));

			return;
		}

		if (planks.empty())
			return;

		auto count = static_cast<int16_t>(planks.size());

		if (keycode == KeyAction::Id::LEFT || keycode == KeyAction::Id::UP)
		{
			focused = focused < 0 ? 0 : (focused + count - 1) % count;
		} else if (keycode == KeyAction::Id::RIGHT || keycode == KeyAction::Id::DOWN || keycode == KeyAction::Id::TAB)
		{
			focused = focused < 0 ? 0 : (focused + 1) % count;
		} else if (keycode == KeyAction::Id::RETURN)
		{
			select_world(static_cast<uint8_t>(focused < 0 ? 0 : focused));
		} else if (escape)
		{
			UI::get().emplace<UIQuitConfirm>();
		}
	}

	UIElement::Type UIWorldSelect::get_type() const
	{
		return TYPE;
	}

	void UIWorldSelect::draw_world()
	{
		nl::node BtWorld = NxFiles::ui_classic["Login.img"]["WorldSelect"]["BtWorld"];

		planks.clear();

		for (size_t i = 0; i < worlds.size() && i < PLANK_SLOTS; i++)
		{
			nl::node src = BtWorld[std::to_string(worlds[i].wid)];

			if (!src)
				src = BtWorld["e"];

			nl::node mouseover = src["mouseOver"];

			Plank plank;
			plank.normal = src["normal"]["0"];
			plank.mouseover = mouseover;
			plank.mouseover_end = mouseover[std::to_string(mouseover.size() - 1)];
			plank.pressed = src["pressed"]["0"];
			plank.focused = src["keyFocused"]["0"];

			planks.push_back(std::move(plank));
		}

		makeactive();
	}

	void UIWorldSelect::add_world(World world)
	{
		worlds.emplace_back(std::move(world));
	}

	void UIWorldSelect::add_recommended_world(RecommendedWorld world)
	{
		recommended_worlds.emplace_back(std::move(world));
	}

	void UIWorldSelect::focus_world(int32_t worldid)
	{
		for (size_t i = 0; i < worlds.size() && i < PLANK_SLOTS; i++)
		{
			if (worlds[i].wid == worldid)
			{
				focused = static_cast<int16_t>(i);
				return;
			}
		}
	}

	void UIWorldSelect::remove_selected()
	{
		deactivate();
	}

	std::string UIWorldSelect::get_world_name(uint8_t worldid) const
	{
		for (auto &world : worlds)
			if (world.wid == worldid)
				return world.name;

		return "";
	}

	uint8_t UIWorldSelect::get_channel_count(uint8_t worldid) const
	{
		for (auto &world : worlds)
			if (world.wid == worldid)
				return world.channelcount;

		return 1;
	}

	Button::State UIWorldSelect::button_pressed(uint16_t id)
	{
		switch (id)
		{
		case Buttons::BT_ENTERWORLD:
			enter_world();
			break;
		case Buttons::BT_BACK:
			// The title logs in afresh on a new connection. Removing a window
			// drops the text focus, so this goes before the title's field
			// takes it.
			Session::get().reconnect();
			UI::get().remove(UIElement::Type::WORLDSELECT);
			UI::get().emplace<UILogin>();
			break;
		default:
			break;
		}

		return Button::State::NORMAL;
	}

	void UIWorldSelect::server_status(uint8_t status)
	{
		// v83 stops waiting before it looks at the status.
		status_pending = false;

		if (requested < 0 || status > 2)
			return;

		int16_t index = requested;
		requested = -1;

		if (status == 1)
			UI::get().emplace<UIOk>("Since There Are Many\\nConcurrent Users in This World,\\nYou May Encounter Some "
				"Difficulties During the Game Play.", [](bool) {});

		if (status == 2)
		{
			UI::get().emplace<UIOk>("The Concurrent Users in This World\\nHave Reached the Max.\\nPlease Try Again Later.",
				[](bool) {});
			return;
		}

		selected = index;

		const World &world = worlds[selected];
		world_name = NxFiles::ui_classic["Login.img"]["WorldSelect"]["world"][std::to_string(world.wid)];

		if (channelid >= world.channelcount)
			channelid = 0;

		channel_select.reset();
		channel_select_done = false;

		if (scroll != Scroll::OPEN)
		{
			// v83 unrolls the channel panel with RollDown (FUN_00629348) and rolls it up with RollUp.
			Sound(Sound::Name::ROLLDOWN).play();

			scroll = Scroll::OPENING;
			scroll_open.reset();
		}
	}

	void UIWorldSelect::select_world(uint8_t index)
	{
		if (status_pending || index >= worlds.size() || index == selected)
			return;

		Sound(Sound::Name::WORLDSELECT).play();

		requested = index;
		focused = index;
		status_pending = true;

		ServerStatusRequestPacket(worlds[index].wid).dispatch();
	}

	void UIWorldSelect::select_channel(uint8_t channel)
	{
		if (status_pending)
			return;

		channelid = channel;
		channel_select.reset();
		channel_select_done = false;
	}

	void UIWorldSelect::close_scroll()
	{
		if (scroll == Scroll::CLOSED || scroll == Scroll::CLOSING)
			return;

		Sound(Sound::Name::ROLLUP).play();

		selected = -1;
		scroll = Scroll::CLOSING;
		scroll_close.reset();
		buttons[Buttons::BT_ENTERWORLD]->set_active(false);
	}

	void UIWorldSelect::enter_world()
	{
		if (scroll != Scroll::OPEN || status_pending)
			return;

		auto worldid = static_cast<uint8_t>(worlds[selected].wid);

		Configuration::get().set_worldid(worldid);
		Configuration::get().set_channelid(channelid);

		UI::get().emplace<UILoginWait>();
		auto loginwait = UI::get().get_element<UILoginWait>();

		if (loginwait && loginwait->is_active())
			CharlistRequestPacket(worldid, channelid).dispatch();
	}

	int16_t UIWorldSelect::plank_at(Point<int16_t> cursorpos) const
	{
		for (size_t i = 0; i < planks.size(); i++)
		{
			Point<int16_t> lt = position + plank_pos(i);
			auto bounds = Rectangle<int16_t>(lt, lt + Point<int16_t>(PLANK_WIDTH, PLANK_HEIGHT));

			if (bounds.contains(cursorpos))
				return static_cast<int16_t>(i);
		}

		return -1;
	}

	int16_t UIWorldSelect::channel_at(Point<int16_t> cursorpos) const
	{
		if (scroll != Scroll::OPEN)
			return -1;

		uint8_t count = std::min<uint8_t>(worlds[selected].channelcount, CHANNEL_SLOTS);

		for (uint8_t i = 0; i < count; i++)
		{
			Point<int16_t> lt = position + channel_pos(i);
			auto bounds = Rectangle<int16_t>(lt, lt + Point<int16_t>(CHANNEL_WIDTH, CHANNEL_HEIGHT));

			if (bounds.contains(cursorpos))
				return i;
		}

		return -1;
	}
}
