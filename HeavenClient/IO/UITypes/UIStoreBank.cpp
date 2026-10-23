#include "UIStoreBank.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/Charset.h"
#include "../Components/MapleButton.h"
#include "../Data/ItemData.h"

#include "../Net/Handlers/StoreBank.h"
#include "../Net/Packets/NpcInteractionPackets.h"
#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The v83 dialog's layout (0x79a0ff and its Draw, 0x79aba0).
		constexpr Point<int16_t> EXIT_BUTTON = Point<int16_t>(154, 15);
		constexpr Point<int16_t> GET_ALL_BUTTON = Point<int16_t>(154, 35);
		constexpr Point<int16_t> NPC_FEET = Point<int16_t>(53, 76);
		// The right end of the mesos.
		constexpr Point<int16_t> MESO = Point<int16_t>(195, 295);
		constexpr int16_t DRAG_HEIGHT = 64;

		// A row's plate, 40 apart; within it the icon's and the cash mark's
		// origins, the name and the count.
		constexpr Point<int16_t> FIRST_ROW = Point<int16_t>(7, 88);
		constexpr int16_t ROW_PITCH = 40;
		constexpr Point<int16_t> ICON = Point<int16_t>(3, 34);
		constexpr Point<int16_t> CASH_MARK = Point<int16_t>(33, 32);
		constexpr Point<int16_t> NAME = Point<int16_t>(44, 3);
		constexpr size_t NAME_WIDTH = 160;
		constexpr Point<int16_t> COUNT = Point<int16_t>(1, 20);

		constexpr int16_t SLIDER_X = 210;
		constexpr Range<int16_t> SLIDER = Range<int16_t>(88, 272);

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}
	}

	UIStoreBank::UIStoreBank() : UIDragElement<PosSTOREBANK>(), npcid(0), offset(0), meso(0), pending(false)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["StoreBank"];

		Texture bg = src["backgrnd"];
		sprites.emplace_back(src["backgrnd"]);

		buttons[Buttons::EXIT] = std::make_unique<MapleButton>(src["BtExit"], EXIT_BUTTON);
		buttons[Buttons::GET_ALL] = std::make_unique<MapleButton>(src["BtGetAll"], GET_ALL_BUTTON);

		enabled_row = src["en"];
		cash_mark = NxFiles::ui_classic["CashShop.img"]["CashItem"]["0"];

		meso_label = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);

		slider = Slider(
				Slider::Type::CLASSIC, SLIDER, SLIDER_X, store_bank::ROWS, store_bank::ROWS,
				[&](bool upwards)
				{
					int16_t next = offset + (upwards ? -1 : 1);

					if (next >= 0 && next <= store_bank::max_offset(rows.size()))
						offset = next;
				}
		);

		active = false;
		dimension = bg.get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), DRAG_HEIGHT);
	}

	void UIStoreBank::draw(float alpha) const
	{
		static const Charset countset = Charset(NxFiles::ui_classic["Basic.img"]["ItemNo"], Charset::Alignment::LEFT);

		UIElement::draw_sprites(alpha);

		npc.draw(DrawArgument(position + NPC_FEET, true));

		for (int16_t i = 0; i < store_bank::ROWS; i++)
		{
			auto index = static_cast<size_t>(offset + i);

			if (index >= rows.size())
				break;

			const Row &row = rows[index];
			Point<int16_t> origin = position + FIRST_ROW + Point<int16_t>(0, ROW_PITCH * i);

			enabled_row.draw(origin);
			row.icon.draw(origin + ICON);

			if (row.cash)
				cash_mark.draw(origin + CASH_MARK);

			row.name.draw(origin + NAME);

			if (store_bank::shows_count(row.itemid))
				countset.draw(std::to_string(row.count), origin + COUNT);
		}

		meso_label.draw(position + MESO);
		slider.draw(position);

		UIElement::draw_buttons(alpha);
	}

	Button::State UIStoreBank::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::EXIT:
				close();
				break;
			case Buttons::GET_ALL:
				// 0x79b21a: one Get All at a time.
				if (!pending)
				{
					pending = true;
					StoreBankRequestPacket(store_bank::Request::GET_ALL).dispatch();
				}

				break;
		}

		return Button::State::NORMAL;
	}

	void UIStoreBank::remove_cursor()
	{
		UIDragElement::remove_cursor();

		slider.remove_cursor();
	}

	Cursor::State UIStoreBank::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (slider.isenabled())
		{
			Cursor::State state = slider.send_cursor(cursorpos - position, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}

		return UIDragElement::send_cursor(clicked, cursorpos);
	}

	void UIStoreBank::send_scroll(double yoffset)
	{
		slider.send_scroll(yoffset);
	}

	void UIStoreBank::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			close();
	}

	UIElement::Type UIStoreBank::get_type() const
	{
		return TYPE;
	}

	void UIStoreBank::open(int32_t id, const storage_contents::Contents &contents)
	{
		npcid = id;
		npc = nl::nx::npc[string_format::extend_id(npcid, 7) + ".img"]["stand"]["0"];

		meso = contents.meso.value_or(0);
		rows.clear();

		for (const storage_contents::Stored &item : store_bank::kept(contents))
		{
			const ItemData &data = ItemData::get(item.itemid);
			Text name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, data.get_name());
			string_format::format_with_ellipsis(name, NAME_WIDTH);

			rows.push_back({ item.itemid, item.count, item.cash, data.get_icon(false), name });
		}

		offset = 0;
		pending = false;

		for (auto &button : buttons)
			button.second->set_state(Button::State::NORMAL);

		refresh();
		makeactive();
	}

	void UIStoreBank::quote(int32_t days, int32_t fee)
	{
		// 0x79b4c4: only a Get All is quoted.
		if (!is_active() || !pending)
			return;

		UI::get().emplace<UIYesNo>(store_bank::fee_text(days, fee),
			[this](bool yes)
			{
				if (yes)
					StoreBankRequestPacket(store_bank::Request::CONFIRM).dispatch();
				else
					pending = false;
			});
	}

	void UIStoreBank::answered(int8_t code)
	{
		if (!is_active() || !pending)
			return;

		pending = false;

		// 0x79bae1: it all went to the player.
		if (code == store_bank::RETRIEVED)
		{
			rows.clear();
			meso = 0;
			offset = 0;
			buttons[Buttons::GET_ALL]->set_state(Button::State::DISABLED);
			refresh();
		}

		notice(store_bank::message_text(code));
	}

	void UIStoreBank::answered_nothing()
	{
		pending = false;
	}

	void UIStoreBank::refresh()
	{
		std::string text = std::to_string(meso);
		string_format::split_number(text);
		meso_label.change_text(text);

		auto size = static_cast<int16_t>(rows.size());
		slider.setrows(offset, store_bank::ROWS, std::max<int16_t>(size, store_bank::ROWS));
	}

	void UIStoreBank::close()
	{
		deactivate();
		StoreBankRequestPacket(store_bank::Request::CLOSE).dispatch();
	}
}
