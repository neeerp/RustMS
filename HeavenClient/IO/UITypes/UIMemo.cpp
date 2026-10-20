#include "UIMemo.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// v83 places text about 3 px lower than ours.
		constexpr int16_t TEXT_RAISE = 3;
		const Point<int16_t> HEADING_AT(100, 32);
		// The list's 260 px wide view; rows wrap their text at 244 px.
		const Point<int16_t> LIST_AT(8, 66);
		constexpr int16_t WRAP = 244;
		const Point<int16_t> SENDER_AT(10, 3), DATE_AT(169, 6);
		constexpr int16_t TEXT_X = 8, TEXT_Y = 3;
		// FontTime's digits are a pixel apart; it has no space, which is 5.
		constexpr int16_t DIGIT_GAP = 1, SPACE = 5;
		constexpr int16_t SLIDER_X = 273;
		const Point<int16_t> OK_AT(241, 216), CLOSE_AT(276, 6);

		constexpr const char* DELETE_WARNING = "All unsaved notes will be deleted.\\nAre you sure you want to move on?";
		constexpr const char* ETC_FULL = "Your Etc slot is full.\\nPlease make some space\\nto receive Wedding Invites.";

		int8_t free_etc_slots()
		{
			const Inventory& inventory = Stage::get().get_player().get_inventory();
			int8_t free = 0;

			for (int16_t slot = 1; slot <= inventory.get_slotmax(InventoryType::Id::ETC); slot++)
				if (inventory.get_item_id(InventoryType::Id::ETC, slot) == 0)
					free++;

			return free;
		}
	}

	UIMemo::UIMemo(std::vector<Memos::Memo> notes) : UIElement(Point<int16_t>(), Point<int16_t>()), memos(std::move(notes))
	{
		nl::node memo = NxFiles::ui_classic["UIWindow.img"]["Memo"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];

		background = memo["backgrnd"];
		inner_top = memo["innerTop"];
		inner_center = memo["innerCenter"];
		inner_bottom = memo["innerBottom"];
		line = memo["line"];

		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
				(Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		buttons[OK] = std::make_unique<MapleButton>(basic["BtOK2"], OK_AT);
		buttons[CLOSE] = std::make_unique<MapleButton>(basic["BtClose"], CLOSE_AT);

		heading = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::MEMOCYAN, "YOU HAVE RECEIVED ");
		count = Text(Text::Font::A11B, Text::Alignment::LEFT, Color::Name::MEMOCYAN, std::to_string(memos.size()));
		tail = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::MEMOCYAN, " NOTE(S).");

		nl::node font = memo["FontTime"];
		const std::string& own = Stage::get().get_player().get_stats().get_name();
		int16_t y = 0;

		for (const Memos::Memo& note : memos)
		{
			Row row;
			row.sender = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::MEMOBLUE, note.sender);
			row.flag = note.flag;

			int16_t x = 0;

			for (char c : Memos::date(note.sent))
			{
				if (c == ' ')
				{
					x += SPACE + DIGIT_GAP;
					continue;
				}

				Texture glyph = font[std::string(1, c)];
				row.date.push_back({ glyph, x });
				x += glyph.width() + DIGIT_GAP;
			}

			auto width = [](const std::string& text)
			{
				return Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, text).width();
			};

			for (const std::string& text : Memos::wrap(Memos::shown_text(note), WRAP, width))
				row.lines.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, text);

			// The server's sender may lack the space before "has".
			if (note.flag == Memos::FAME || note.flag == Memos::GIFT)
			{
				row.extra.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
						note.sender + " has received a gift.");
				row.extra.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
						own + "'s fame has gone up +1.");
			}

			row.y = y;
			row.height = Memos::row_height(row.lines.size(), note.flag);
			y += row.height;

			rows.push_back(std::move(row));
		}

		slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(LIST_AT.y(), LIST_AT.y() + Memos::VIEW), SLIDER_X, 1,
				Memos::scroll_steps(y) + 1, [&](bool up) { scroll_to(step + (up ? -1 : 1)); });
	}

	void UIMemo::open()
	{
		if (Memos::Inbox::get().empty())
			return;

		UI::get().emplace<UIMemo>(Memos::Inbox::get().take());
	}

	void UIMemo::draw(float inter) const
	{
		background.draw(position);

		Point<int16_t> raise(0, TEXT_RAISE);
		Point<int16_t> heading_at = position + HEADING_AT - raise;
		heading.draw(heading_at);
		count.draw(heading_at + Point<int16_t>(heading.width(), 0));
		tail.draw(heading_at + Point<int16_t>(heading.width() + count.width(), 0));

		Range<int16_t> shown = view();
		int16_t scrolled = step * Memos::STEP;

		for (size_t i = 0; i < rows.size(); i++)
		{
			const Row& row = rows[i];
			int16_t top = row.y - scrolled;

			if (top >= Memos::VIEW)
				break;

			if (top + row.height <= 0)
				continue;

			Point<int16_t> at = position + LIST_AT + Point<int16_t>(0, top);
			draw_clipped(inner_top, at);
			row.sender.draw(DrawArgument(at + SENDER_AT - raise), shown);

			for (const Glyph& glyph : row.date)
				draw_clipped(glyph.texture, at + DATE_AT + Point<int16_t>(glyph.x, 0));

			int16_t y = Memos::TOP;
			size_t centers = row.lines.size() + (row.flag == Memos::FAME ? 3 : 0);

			for (size_t l = 0; l < centers; l++)
				draw_clipped(inner_center, at + Point<int16_t>(0, Memos::TOP + Memos::LINE * l));

			for (const Text& text : row.lines)
			{
				text.draw(DrawArgument(at + Point<int16_t>(TEXT_X, y + TEXT_Y) - raise), shown);
				y += Memos::LINE;
			}

			for (const Text& text : row.extra)
			{
				text.draw(DrawArgument(at + Point<int16_t>(TEXT_X, y + TEXT_Y) - raise), shown);
				y += Memos::LINE;
			}

			int16_t bottom = Memos::TOP + Memos::LINE * centers;
			draw_clipped(inner_bottom, at + Point<int16_t>(0, bottom));

			if (i + 1 < rows.size())
				draw_clipped(line, at + Point<int16_t>(0, row.height - (Memos::GAP + 1) / 2));
		}

		slider.draw(position);

		UIElement::draw(inter);
	}

	void UIMemo::draw_clipped(const Texture& texture, Point<int16_t> at) const
	{
		Range<int16_t> shown = view();
		Point<int16_t> size = texture.get_dimensions();
		int16_t first = std::max<int16_t>(0, shown.first() - at.y());
		int16_t last = std::min<int16_t>(size.y(), shown.second() - at.y());

		if (first >= last)
			return;

		texture.draw(DrawArgument(at), Rectangle<int16_t>(0, size.x(), first, last));
	}

	Range<int16_t> UIMemo::view() const
	{
		int16_t top = position.y() + LIST_AT.y();

		return Range<int16_t>(top, top + Memos::VIEW);
	}

	void UIMemo::update()
	{
		UIElement::update();
	}

	void UIMemo::scroll_to(int16_t to)
	{
		int16_t total = rows.empty() ? 0 : rows.back().y + rows.back().height;
		step = std::clamp<int16_t>(to, 0, Memos::scroll_steps(total));
	}

	Cursor::State UIMemo::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (Cursor::State state = slider.send_cursor(cursorpos - position, clicked))
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIMemo::send_scroll(double yoffset)
	{
		slider.send_scroll(yoffset);
	}

	void UIMemo::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && escape)
			ask_to_close();
	}

	Button::State UIMemo::button_pressed(uint16_t)
	{
		ask_to_close();

		return Button::State::NORMAL;
	}

	void UIMemo::ask_to_close()
	{
		if (warned)
			return;

		warned = true;

		// No leaves the inbox open.
		UI::get().emplace<UIYesNo>(DELETE_WARNING, [](bool yes)
		{
			auto inbox = UI::get().get_element<UIMemo>();

			if (!inbox)
				return;

			inbox->warned = false;

			if (yes)
				inbox->finish();
		});
	}

	void UIMemo::finish()
	{
		int8_t free = free_etc_slots();
		auto invitations = std::count_if(memos.begin(), memos.end(),
				[](const Memos::Memo& memo) { return memo.flag == Memos::INVITATION; });

		ReadNotesPacket(memos, free).dispatch();

		if (invitations > free)
			UI::get().emplace<UIOk>(ETC_FULL, [](bool) {});

		deactivate();
	}

	UIElement::Type UIMemo::get_type() const
	{
		return TYPE;
	}
}
