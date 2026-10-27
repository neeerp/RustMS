#include "UIRaise.h"

#include "UIChatBar.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The window's size until there is art to show (0x8938b7), and where it first opens (0x8937fe).
		constexpr Point<int16_t> FIRST_SIZE = Point<int16_t>(150, 150);
		constexpr Point<int16_t> FIRST_POSITION = Point<int16_t>(0, 150);
		// The art's place from the top left (0x892635); the window is the art and 20 by 40 more (0x89269a).
		// The art's origin isn't taken off, as the window's size wouldn't hold it if it were.
		constexpr Point<int16_t> ART = Point<int16_t>(10, 30);
		constexpr Point<int16_t> MARGINS = Point<int16_t>(20, 40);
		// The close button's place from the top right (0x89212d).
		constexpr int16_t CLOSE_RIGHT = 16;
		constexpr int16_t CLOSE_TOP = 4;

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}

		void chat_line(const std::string &text)
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(text, UIChatBar::LineType::SKILL);
		}

		// An item's info node, for the items a raise item takes and the raise item itself.
		nl::node item_info(int32_t item_id)
		{
			const char *category;

			switch (item_id / 1000000)
			{
				case 2:
					category = "Consume";
					break;
				case 3:
					category = "Install";
					break;
				case 4:
					category = "Etc";
					break;
				default:
					return nl::node();
			}

			std::string image = "0" + std::to_string(item_id / 10000) + ".img";

			return nl::nx::item[category][image]["0" + std::to_string(item_id)]["info"];
		}

		// `texture` drawn with its top left at `at`: the window's canvases are put without their origins.
		void put(const Texture &texture, Point<int16_t> at, Point<int16_t> stretch = Point<int16_t>())
		{
			texture.draw(DrawArgument(at + texture.get_origin(), stretch));
		}
	}

	std::map<int32_t, Point<int16_t>> UIRaise::positions;

	void UIRaise::open(int32_t item_id)
	{
		auto window = UI::get().get_element<UIRaise>();

		if (window && window->is_active() && window->item_id == item_id)
		{
			UI::get().raise(TYPE);

			return;
		}

		// The factory (0x8932b8) reads the item's info and opens nothing without a record, art or grade.
		nl::node info = item_info(item_id);
		int16_t record = static_cast<int16_t>(info["questId"].get_integer());
		auto [file, path] = raise::ui_path(info["uiData"]);
		raise::Growth growth{info["exp"], info["grade"]};

		if (record == 0 || file.empty() || !growth.valid())
			return;

		nl::node art = NxFiles::ui_classic[file].resolve(path);
		std::vector<std::string> messages;

		for (int32_t i = 0; info["message"][std::to_string(i)]; i++)
			messages.push_back(info["message"][std::to_string(i)].get_string());

		std::vector<int32_t> feed;

		for (int32_t i = 0; int32_t id = info["consumeItem"][std::to_string(i)]["0"]; i++)
			feed.push_back(id);

		// HeavenClient shows one raise window at a time; v83 one per item.
		if (window && window->is_active())
			window->close();

		UI::get().emplace<UIRaise>(item_id, record, growth, art, messages, feed);
	}

	void UIRaise::record_changed(int16_t record, int8_t status, const std::string &value)
	{
		auto window = UI::get().get_element<UIRaise>();

		if (!window || !window->is_active() || window->record != record)
			return;

		if (status == 1)
			window->set_value(std::atoi(value.c_str()));
		else
			window->close();
	}

	UIRaise::UIRaise(int32_t id, int16_t rec, raise::Growth g, nl::node a, std::vector<std::string> m,
					 std::vector<int32_t> f)
			: UIElement(Point<int16_t>(), FIRST_SIZE), item_id(id), record(rec), growth(g), art_src(a),
			  messages(std::move(m)), feed(std::move(f)), value(g.unknown()), dragged(false)
	{
		nl::node backgrnd = NxFiles::ui_classic["UIWindow.img"]["raise"]["backgrnd"];
		const char *rows[] = {"top", "center", "bottom"};
		const char *columns[] = {"left", "center", "right"};

		for (int row = 0; row < 3; row++)
			for (int column = 0; column < 3; column++)
				frame[row][column] = backgrnd[rows[row]][columns[column]];

		// UI/Basic.img/BtClose, not the one under raise (0x891070).
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"]);
		resize();

		auto saved = positions.find(item_id);
		position = saved == positions.end() ? FIRST_POSITION : saved->second;

		RaiseWindowPacket(item_id, true).dispatch();
		RaiseRecordPacket(record).dispatch();
	}

	void UIRaise::draw(float alpha) const
	{
		int16_t left = frame[0][0].width();
		int16_t right = frame[0][2].width();
		int16_t top = frame[0][0].height();
		int16_t bottom = frame[2][0].height();
		int16_t middle_width = dimension.x() - left - right;
		int16_t heights[] = {top, static_cast<int16_t>(dimension.y() - top - bottom), bottom};
		int16_t y = 0;

		// Compose (0x89191b): the corners as they are, the edges and middle stretched.
		for (int row = 0; row < 3; row++)
		{
			put(frame[row][0], position + Point<int16_t>(0, y), Point<int16_t>(left, heights[row]));
			put(frame[row][1], position + Point<int16_t>(left, y), Point<int16_t>(middle_width, heights[row]));
			put(frame[row][2], position + Point<int16_t>(dimension.x() - right, y), Point<int16_t>(right, heights[row]));
			y += heights[row];
		}

		put(art, position + ART);

		UIElement::draw(alpha);
	}

	void UIRaise::remove_cursor()
	{
		UIElement::remove_cursor();

		dragged = false;
	}

	Cursor::State UIRaise::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (dragged)
		{
			if (clicked)
			{
				position = cursorpos - drag_from;

				return Cursor::State::CLICKING;
			}

			dragged = false;
		}

		Point<int16_t> cursor = cursorpos - position;
		Cursor::State ret = UIElement::send_cursor(clicked, cursorpos);

		// By the frame's top row, as other windows by their title bars.
		if (clicked && cursor.y() >= 0 && cursor.y() < frame[0][0].height() && !is_holding_button())
		{
			drag_from = cursor;
			dragged = true;
		}

		return ret;
	}

	bool UIRaise::send_icon(const Icon &icon, Point<int16_t>)
	{
		// An item dropped on the window (0x4f4a17): what it doesn't take gets a notice (0x892878). What it does
		// shows the item's message 1 in the chat log and is sent, or once grown its message 0 as a notice.
		auto [tab, slot] = icon.get_source();

		if (tab == InventoryType::Id::NONE)
			return true;

		Player &player = Stage::get().get_player();
		int32_t fed = player.get_inventory().get_item_id(tab, slot);

		if (std::find(feed.begin(), feed.end(), fed) == feed.end())
		{
			notice(raise::NOT_AVAILABLE);

			return true;
		}

		int32_t which = growth.feed_message(value);
		std::string message = item_info(fed)["message"][std::to_string(which)];

		if (which == 0)
		{
			notice(message);

			return true;
		}

		chat_line(message);

		// 0x8913a8: once a request is answered and 200 ms have passed; the record answers it.
		player.request(RaiseFeedPacket(tab, slot, item_id), ExclRequest::USE_WAIT);

		return true;
	}

	UIElement::Type UIRaise::get_type() const
	{
		return TYPE;
	}

	Button::State UIRaise::button_pressed(uint16_t)
	{
		close();

		return Button::State::NORMAL;
	}

	void UIRaise::set_value(int32_t v)
	{
		// 0x891442: a new stage's art, and its message in the chat log (0x89216e).
		int32_t stage = growth.stage(value);
		value = v;
		int32_t now = growth.stage(value);

		if (now == stage)
			return;

		art = now >= 0 ? Texture(art_src[std::to_string(now)]) : Texture();
		resize();

		if (now >= 0 && now < static_cast<int32_t>(messages.size()))
			chat_line(messages[now]);
	}

	void UIRaise::resize()
	{
		if (art.is_valid())
			dimension = art.get_dimensions() + MARGINS;

		buttons[Buttons::CLOSE]->set_position(Point<int16_t>(dimension.x() - CLOSE_RIGHT, CLOSE_TOP));
	}

	void UIRaise::close()
	{
		// The window's destructor tells the server (0x890f79), and its place is kept (0x893019).
		positions[item_id] = position;
		RaiseWindowPacket(item_id, false).dispatch();
		deactivate();
	}
}
