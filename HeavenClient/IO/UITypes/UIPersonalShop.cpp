#include "UIPersonalShop.h"

#include "UICharInfo.h"
#include "UIChatBar.h"
#include "UIItemInventory.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/Charset.h"
#include "../Components/MapleButton.h"

#include "../../Character/ExclRequest.h"
#include "../../Character/Inventory/ItemProperties.h"
#include "../../Constants.h"
#include "../../Data/CurseData.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/Geometry.h"
#include "../../Net/Packets/MiniRoomPackets.h"
#include "../../Net/Packets/PlayerInteractionPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <algorithm>
#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		using personal_shop::OWNER;
		using personal_shop::SEATS;

		// The v83 dialog's controls and drawing (FUN_006fa4b2, FUN_006fb99d).
		constexpr Point<int16_t> BT_TOP = Point<int16_t>(144, 62);
		constexpr Point<int16_t> BT_MIDDLE = Point<int16_t>(144, 90);
		constexpr Point<int16_t> BT_BOTTOM = Point<int16_t>(144, 110);
		constexpr Point<int16_t> BT_ENTER = Point<int16_t>(482, 321);
		constexpr Point<int16_t> BT_CLAIM = Point<int16_t>(498, 343);
		constexpr int16_t BT_BAN_Y = 117;

		// v83 draws a line's cap top about 3 below where it is drawn, we 9,
		// so all text here goes up by the difference.
		constexpr Point<int16_t> TEXT_Y = Point<int16_t>(0, -6);

		constexpr Point<int16_t> TITLE = Point<int16_t>(23, 9);
		constexpr size_t TITLE_WIDTH = 496;
		constexpr Point<int16_t> MESO = Point<int16_t>(218, 135);

		// The owner stands on the left facing right, the visitors to the
		// right of them facing left; names go on the tags below.
		constexpr Point<int16_t> OWNER_FEET = Point<int16_t>(53, 127);
		constexpr int16_t OWNER_NAME_X = 56;
		constexpr int16_t FEET_Y = 127;
		constexpr int16_t NAME_Y = 135;
		constexpr int16_t SEAT_PITCH = 95;

		int16_t seat_x(uint8_t slot)
		{
			return (slot + 2) * SEAT_PITCH;
		}

		// Five rows of items, 40 apart.
		constexpr int16_t ROW_TOP = 157;
		constexpr int16_t ROW_PITCH = 40;
		constexpr int16_t ROWS = 5;
		constexpr int16_t ROW_LEFT = 7;
		constexpr Point<int16_t> ROW_SIZE = Point<int16_t>(199, 35);
		constexpr Point<int16_t> SELECT = Point<int16_t>(44, 0);
		constexpr Point<int16_t> ICON = Point<int16_t>(8, 34);
		constexpr Point<int16_t> COUNT = Point<int16_t>(8, 23);
		constexpr Point<int16_t> NAME = Point<int16_t>(48, 1);
		constexpr Point<int16_t> PRICE = Point<int16_t>(48, 19);
		constexpr size_t NAME_WIDTH = 152;
		constexpr int16_t ITEMS_X = 211;
		const Range<int16_t> ITEMS_RANGE = Range<int16_t>(158, 355);

		// The chat log: nine rows of 16, wrapped to 243.
		constexpr int16_t LOG_LEFT = 241;
		constexpr int16_t LOG_TOP = 162;
		constexpr int16_t LOG_ROW = 16;
		constexpr int16_t LOG_ROWS = 9;
		constexpr int16_t LOG_WIDTH = 243;
		constexpr size_t LOG_LIMIT = 200;
		constexpr int16_t LOG_CLIP_TOP = 160;
		constexpr int16_t LOG_CLIP_BOTTOM = 321;
		constexpr int16_t CHAT_X = 502;
		const Range<int16_t> CHAT_RANGE = Range<int16_t>(157, 318);

		const Rectangle<int16_t> INPUT = Rectangle<int16_t>(238, 469, 323, 338);
		constexpr size_t INPUT_LIMIT = 70;

		// Clicks pass through the empty corner right of the fee table.
		constexpr Point<int16_t> CLEAR_CORNER = Point<int16_t>(231, 367);
		constexpr int16_t DRAG_HEIGHT = 156;

		// A line takes the colour of its speaker's name tag.
		constexpr Color::Name SEAT_COLORS[SEATS] = {
			Color::Name::BLACK, Color::Name::SEATBLUE, Color::Name::SEATPINK, Color::Name::DEEPGREEN
		};

		constexpr Color::Name PRICE_COLORS[] = {
			Color::Name::PRICEBLUE, Color::Name::PRICETEAL, Color::Name::PRICEGREEN,
			Color::Name::PRICEORANGE, Color::Name::PRICEPINK, Color::Name::PRICEPLUM
		};

		const std::string FULL = "You can't sell it anymore.";
		const std::string TOO_PRICEY = "The total price for the items exceeded\\nthe maximum amount of mesos that can be sold.";

		Point<int16_t> centred(Point<int16_t> dimension)
		{
			return Point<int16_t>(
					(Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
					(Constants::Constants::get().get_viewheight() - dimension.y()) / 2
			);
		}

		void notice(const std::string &text)
		{
			UI::get().emplace<UIOk>(text, [](bool) {});
		}
	}

	UIPersonalShop::UIPersonalShop(const mini_room::Enter &enter) : UIDragElement<PosPERSONALSHOP>(),
			own_slot(enter.own_slot < SEATS ? enter.own_slot : OWNER), max_items(enter.max_items), open(own_slot != OWNER), selected(-1),
			first_row(0), log_rows(0), log_row(0)
	{
		nl::node PersonalShop = NxFiles::ui_classic["UIWindow.img"]["PersonalShop"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = PersonalShop["backgrnd6"];
		select = PersonalShop["select"];

		// Each side gets only its own buttons (FUN_006fc46b).
		if (is_owner())
		{
			buttons[Buttons::ITEM] = std::make_unique<MapleButton>(PersonalShop["BtItem"], BT_TOP);
			buttons[Buttons::START] = std::make_unique<MapleButton>(PersonalShop["BtStart"], BT_MIDDLE);
			buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(PersonalShop["BtClose"], BT_BOTTOM);

			// A ban button shows over each visitor while they stay.
			for (uint8_t slot = 1; slot < SEATS; slot++)
			{
				uint16_t id = Buttons::BAN1 + slot - 1;
				buttons[id] = std::make_unique<MapleButton>(PersonalShop["BtBan"], Point<int16_t>(seat_x(slot) + 30, BT_BAN_Y));
				buttons[id]->set_active(false);
			}
		} else
		{
			buttons[Buttons::INFO] = std::make_unique<MapleButton>(PersonalShop["BtInfo"], BT_TOP);
			buttons[Buttons::BUY] = std::make_unique<MapleButton>(PersonalShop["BtBuy"], BT_MIDDLE);
			buttons[Buttons::EXIT] = std::make_unique<MapleButton>(PersonalShop["BtExit"], BT_BOTTOM);
		}

		buttons[Buttons::ENTER] = std::make_unique<MapleButton>(PersonalShop["BtEnter"], BT_ENTER);
		buttons[Buttons::CLAIM] = std::make_unique<MapleButton>(Basic["BtClaim"], BT_CLAIM);

		title = Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::BLACK, enter.title);
		string_format::format_with_ellipsis(title, TITLE_WIDTH);

		meso = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);

		for (Member &member : members)
			member.name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::WHITE);

		for (const mini_room::Member &member : enter.members)
			visit(member);

		items = Slider(
				Slider::Type::CLASSIC, ITEMS_RANGE, ITEMS_X, ROWS, 0,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;

					if (first_row + shift >= 0 && first_row + shift + ROWS <= static_cast<int16_t>(listings.size()))
						first_row += shift;
				}
		);

		chat = Slider(
				Slider::Type::CLASSIC, CHAT_RANGE, CHAT_X, LOG_ROWS, 0,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;

					if (log_row + shift >= 0 && log_row + shift + LOG_ROWS <= log_rows)
						log_row += shift;
				}
		);

		input = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, INPUT, INPUT_LIMIT);

		input.set_enter_callback(
				[&](std::string text)
				{
					input.change_text("");
					submit(text);
				}
		);

		input.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					input.set_state(Textfield::State::DISABLED);
					input.set_state(Textfield::State::NORMAL);
				}
		);

		set_listings(enter.listings);

		dimension = backgrnd.get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), DRAG_HEIGHT);
	}

	void UIPersonalShop::draw(float inter) const
	{
		static const Charset countset = Charset(NxFiles::ui_classic["Basic.img"]["ItemNo"], Charset::Alignment::LEFT);

		backgrnd.draw(position);
		title.draw(position + TITLE + TEXT_Y);

		for (uint8_t slot = 0; slot < SEATS; slot++)
		{
			const Member &member = members[slot];

			if (!member.present)
				continue;

			// Our own avatar follows our equipment as it changes.
			const CharLook &look = slot == own_slot ? Stage::get().get_player().get_look() : member.look;

			if (slot == OWNER)
			{
				look.draw(position + OWNER_FEET, true, Stance::Id::STAND1, Expression::Id::DEFAULT);
				member.name.draw(position + Point<int16_t>(OWNER_NAME_X, NAME_Y) + TEXT_Y);
			} else
			{
				look.draw(position + Point<int16_t>(seat_x(slot), FEET_Y), false, Stance::Id::STAND1, Expression::Id::DEFAULT);
				member.name.draw(position + Point<int16_t>(seat_x(slot) - 2, NAME_Y) + TEXT_Y);
			}
		}

		meso.draw(position + MESO + TEXT_Y);

		for (int16_t k = 0; k < ROWS; k++)
		{
			auto index = static_cast<size_t>(first_row + k);

			if (index >= listings.size())
				break;

			const mini_room::Listing &listing = listings[index];
			int32_t itemid = listing.item.itemid;
			Point<int16_t> row = position + Point<int16_t>(0, ROW_TOP + ROW_PITCH * k);

			// A sold out row stays selected but loses its highlight.
			if (static_cast<int16_t>(index) == selected && listing.bundles != 0)
				select.draw(row + SELECT);

			ItemData::get(itemid).get_icon(false).draw(row + ICON);

			if (listing.bundles == 0)
				ColorBox(ROW_SIZE.x(), ROW_SIZE.y(), Color::Name::WHITE, 0.5f).draw(row + Point<int16_t>(ROW_LEFT, 0));

			if (personal_shop::shows_count(itemid))
				countset.draw(std::to_string(listing.bundles * listing.per_bundle), row + COUNT);

			rows[index].name.draw(row + NAME + TEXT_Y);
			rows[index].price.draw(row + PRICE + TEXT_Y);
		}

		items.draw(position);

		Range<int16_t> clip = Range<int16_t>(position.y() + LOG_CLIP_TOP, position.y() + LOG_CLIP_BOTTOM);
		int16_t y = LOG_TOP - log_row * LOG_ROW;

		for (const Line &line : log)
		{
			if (y + line.rows * LOG_ROW > LOG_TOP && y < LOG_TOP + LOG_ROWS * LOG_ROW)
				line.text.draw(DrawArgument(position + Point<int16_t>(LOG_LEFT, y) + TEXT_Y), clip);

			y += line.rows * LOG_ROW;
		}

		chat.draw(position);
		input.draw(position + TEXT_Y);

		UIElement::draw_buttons(inter);
	}

	void UIPersonalShop::update()
	{
		UIElement::update();

		meso.change_text(mini_room::meso_text(Stage::get().get_player().get_inventory().get_meso()));
		input.update(position);
	}

	void UIPersonalShop::remove_cursor()
	{
		UIDragElement::remove_cursor();

		items.remove_cursor();
		chat.remove_cursor();
		clear_tooltip();
	}

	Cursor::State UIPersonalShop::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
		{
			clear_tooltip();

			return dstate;
		}

		Point<int16_t> cursor = cursorpos - position;
		last_cursor = cursor;

		for (Slider *slider : { &items, &chat })
		{
			if (Cursor::State sstate = slider->send_cursor(cursor, clicked))
			{
				clear_tooltip();

				return sstate;
			}
		}

		Cursor::State tstate = input.send_cursor(cursorpos, clicked);

		if (tstate != Cursor::State::IDLE)
			return tstate;

		int16_t index = row_at(cursor);

		if (index < 0)
		{
			clear_tooltip();

			return dstate;
		}

		const mini_room::Listing &listing = listings[index];

		// Sold out rows still show what they were (FUN_006fb864).
		UI::get().show_item(Tooltip::Parent::PERSONALSHOP, listing.item.itemid);

		if (clicked)
		{
			selected = index;

			// Before opening, the owner can take a row back by dragging it
			// to the inventory (FUN_006fb368).
			if (is_owner() && !open && listing.bundles >= 1)
			{
				Point<int16_t> top_left = Point<int16_t>(ICON.x(), ROW_TOP + ROW_PITCH * (index - first_row) + ICON.y() - 32);
				const Texture &icon = ItemData::get(listing.item.itemid).get_icon(false);

				dragged_row = Icon(std::make_unique<RowIcon>(index, listing.item.itemid), icon, -1);
				dragged_row.start_drag(cursor - top_left);
				UI::get().drag_icon(&dragged_row);

				clear_tooltip();

				return Cursor::State::GRABBING;
			}
		}

		return dstate;
	}

	bool UIPersonalShop::send_icon(const Icon &icon, Point<int16_t> cursorpos)
	{
		// Only the owner's own inventory items go in; the item checks follow
		// in the icon (FUN_004f41b9).
		if (is_owner())
			icon.drop_on_shop();

		return true;
	}

	void UIPersonalShop::doubleclick(Point<int16_t> cursorpos)
	{
		int16_t index = row_at(cursorpos - position);

		if (index >= 0)
			buy(index);
	}

	void UIPersonalShop::send_scroll(double yoffset)
	{
		if (last_cursor.x() > ITEMS_X + 20)
			chat.send_scroll(yoffset);
		else
			items.send_scroll(yoffset);
	}

	void UIPersonalShop::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			exit();
	}

	bool UIPersonalShop::is_in_range(Point<int16_t> cursorpos) const
	{
		Point<int16_t> at = cursorpos - position;

		if (at.x() > CLEAR_CORNER.x() && at.y() > CLEAR_CORNER.y())
			return false;

		return UIElement::is_in_range(cursorpos);
	}

	UIElement::Type UIPersonalShop::get_type() const
	{
		return TYPE;
	}

	void UIPersonalShop::visit(const mini_room::Member &member)
	{
		if (member.slot >= members.size())
			return;

		Member &seat = members[member.slot];
		seat.present = true;
		seat.look = CharLook(member.look);
		seat.name.change_text(member.name);

		if (member.slot != OWNER && is_owner())
			buttons[Buttons::BAN1 + member.slot - 1]->set_active(true);
	}

	void UIPersonalShop::seat_left(uint8_t slot)
	{
		if (slot == OWNER || slot >= members.size())
			return;

		members[slot].present = false;
		members[slot].name.change_text("");

		if (is_owner())
			buttons[Buttons::BAN1 + slot - 1]->set_active(false);
	}

	void UIPersonalShop::change_look(uint8_t slot, const LookEntry &look)
	{
		if (slot < members.size() && slot != own_slot && members[slot].present)
			members[slot].look = CharLook(look);
	}

	void UIPersonalShop::add_chat(uint8_t slot, const std::string &line)
	{
		Color::Name color = SEAT_COLORS[slot < SEATS ? slot : OWNER];

		add_line(slot == own_slot ? line : mini_room::filter_chat(line, CurseData::filter), color);
	}

	void UIPersonalShop::set_listings(const std::vector<mini_room::Listing> &list)
	{
		listings = list;

		refresh_rows();
	}

	void UIPersonalShop::sold(const personal_shop::SoldNotice &notice)
	{
		if (notice.index >= listings.size())
			return;

		sales.add(listings[notice.index], notice.bundles, notice.buyer);

		if (auto dialog = UI::get().get_element<UIShopSales>(); dialog && dialog->is_active())
			dialog->refresh();
	}

	void UIPersonalShop::buy_result(int8_t result)
	{
		std::string text = personal_shop::buy_result_text(result);

		if (!text.empty())
			notice(text);
	}

	void UIPersonalShop::leave(int8_t reason)
	{
		std::string text = personal_shop::leave_text(reason);

		close();

		if (!text.empty())
			notice(text);
	}

	void UIPersonalShop::close()
	{
		clear_tooltip();
		UI::get().cancel_drag();

		if (input.get_state() == Textfield::State::FOCUSED)
			input.set_state(Textfield::State::DISABLED);

		input.set_state(Textfield::State::NORMAL);
		input.change_text("");

		for (UIElement::Type type : { UIElement::Type::SHOPSELL, UIElement::Type::SHOPSALES })
			UI::get().remove(type);

		deactivate();
	}

	void UIPersonalShop::sell(InventoryType::Id tab, int16_t slot, int32_t itemid, int16_t count)
	{
		// Nothing more goes in once the shop is full or open (FUN_006fd96c).
		if (listings.size() >= max_items || open)
		{
			notice(FULL);

			return;
		}

		if (auto dialog = UI::get().get_element<UIShopSell>(); dialog && dialog->is_active())
			return;

		// A set of stars or bullets counts as one.
		int16_t quantity = shop_list::is_rechargeable(itemid) ? 1 : count;

		UI::get().emplace<UIShopSell>(tab, slot, itemid, quantity);
	}

	void UIPersonalShop::take_back(int16_t index) const
	{
		if (!is_owner() || open || index < 0 || index >= static_cast<int16_t>(listings.size()))
			return;

		// The row leaves when the server sends the new list.
		if (listings[index].bundles >= 1)
			Stage::get().get_player().request(ShopTakeBackPacket(index), ExclRequest::WAIT);
	}

	const personal_shop::Sales &UIPersonalShop::get_sales() const
	{
		return sales;
	}

	uint8_t UIPersonalShop::get_own_slot() const
	{
		return own_slot;
	}

	void UIPersonalShop::create(int16_t slot, int32_t itemid)
	{
		if (auto shop = UI::get().get_element<UIPersonalShop>(); shop && shop->is_active())
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline("Not available", UIChatBar::LineType::SKILL);

			return;
		}

		if (!Stage::get().get_mapinfo().allows_personal_shop())
		{
			notice("The personal store can only be built\\nat the free market available at main towns.");

			return;
		}

		UI::get().emplace<UIEnterText>(
				"Please enter the name of the store.\\n\\nIf you move before you open the store,\\n the store may not be opened.",
				[slot, itemid](const std::string &title)
				{
					if (title.empty())
						return;

					// v83 (FUN_00a1dd99) opens the store with curse words
					// starred out rather than refusing the name.
					ShopCreatePacket(CurseData::filter(title), slot, itemid).dispatch();
				},
				50
		);
	}

	Button::State UIPersonalShop::button_pressed(uint16_t buttonid)
	{
		clear_tooltip();

		switch (buttonid)
		{
			case Buttons::START:
				// Opening an empty shop does nothing (FUN_006fcc19).
				if (!listings.empty() && !open)
				{
					ShopBlacklistPacket({}).dispatch();
					ShopOpenPacket().dispatch();

					open = true;
					buttons[Buttons::START]->set_state(Button::State::DISABLED);

					return Button::State::DISABLED;
				}

				break;
			case Buttons::BUY:
				buy(selected);
				break;
			case Buttons::CLOSE:
			case Buttons::EXIT:
				exit();

				return Button::State::NORMAL;
			case Buttons::INFO:
				show_owner_info();
				break;
			case Buttons::ITEM:
				if (auto dialog = UI::get().get_element<UIShopSales>(); !dialog || !dialog->is_active())
					UI::get().emplace<UIShopSales>();

				break;
			case Buttons::ENTER:
			{
				std::string text = input.get_text();
				input.change_text("");
				submit(text);
				break;
			}
			case Buttons::BAN1:
			case Buttons::BAN2:
			case Buttons::BAN3:
				ban(static_cast<uint8_t>(buttonid - Buttons::BAN1 + 1));
				break;
			default:
				// Like the status bar's, the claim button does nothing.
				break;
		}

		return Button::State::NORMAL;
	}

	bool UIPersonalShop::is_owner() const
	{
		return own_slot == OWNER;
	}

	int16_t UIPersonalShop::row_at(Point<int16_t> cursor) const
	{
		if (cursor.x() < ROW_LEFT || cursor.x() >= ROW_LEFT + ROW_SIZE.x() || cursor.y() < ROW_TOP)
			return -1;

		int16_t k = (cursor.y() - ROW_TOP) / ROW_PITCH;

		if (k >= ROWS || (cursor.y() - ROW_TOP) % ROW_PITCH >= ROW_SIZE.y())
			return -1;

		int16_t index = first_row + k;

		return index < static_cast<int16_t>(listings.size()) ? index : -1;
	}

	void UIPersonalShop::buy(int16_t index)
	{
		// Only visitors buy, from a row that exists (FUN_006fd261).
		if (is_owner() || index < 0 || index >= static_cast<int16_t>(listings.size()))
			return;

		const mini_room::Listing &listing = listings[index];
		auto row = static_cast<uint8_t>(index);

		if (listing.bundles <= 0)
		{
			notice("The item has been sold already.");

			return;
		}

		// v83 doesn't check our mesos or space; the server answers that.
		auto send = [row](int16_t bundles, int64_t total)
		{
			Player &player = Stage::get().get_player();

			if (personal_shop::over_low_level_limit(player.get_stats().get_stat(MapleStat::Id::LEVEL), total))
			{
				notice(personal_shop::buy_result_text(0xe));

				return;
			}

			player.request(ShopBuyPacket(row, bundles), ExclRequest::WAIT);
		};

		mini_room::Listing chosen = listing;

		auto order = [send, chosen]()
		{
			if (personal_shop::sells_whole(chosen.item.itemid))
			{
				int64_t total = chosen.price;

				UI::get().emplace<UIYesNo>(
						"Are you sure you want to buy it?",
						[send, total](bool yes)
						{
							if (yes)
								send(1, total);
						}
				);

				return;
			}

			UI::get().emplace<UIEnterNumber>(
					"How many are you willing to buy?",
					[send, chosen](int32_t units)
					{
						personal_shop::Purchase purchase = personal_shop::purchase(chosen, units);

						if (!purchase.problem.empty())
						{
							notice(purchase.problem);

							return;
						}

						UI::get().emplace<UIYesNo>(
								personal_shop::total_question(purchase.total),
								[send, purchase](bool yes)
								{
									if (yes)
										send(purchase.bundles, purchase.total);
								}
						);
					},
					chosen.bundles * chosen.per_bundle, 1
			);
		};

		// Buying uses up Scissors of Karma (FUN_006fd261).
		if (!listing.item.cash && item_properties::karma(listing.item.itemid, listing.item.flags))
			UI::get().emplace<UIYesNo>("Once purchased, the item cannot be traded.\\nWould you still like to purchase it?",
				[order](bool yes)
				{
					if (yes)
						order();
				});
		else
			order();
	}

	void UIPersonalShop::ban(uint8_t slot)
	{
		if (slot >= members.size() || !members[slot].present)
			return;

		std::string name = members[slot].name.get_text();

		UI::get().emplace<UIYesNo>(
				"Would you like to ban " + name + "?",
				[slot, name](bool yes)
				{
					if (yes)
						ShopBanPacket(slot, name).dispatch();
				}
		);
	}

	void UIPersonalShop::show_owner_info() const
	{
		// The owner's, not a visitor's (FUN_006fb07b).
		const std::string &name = members[OWNER].name.get_text();

		for (auto &mmo : *Stage::get().get_chars().get_chars())
		{
			auto character = static_cast<const Char *>(mmo.second.get());

			if (character->get_name() == name)
			{
				int32_t cid = character->get_oid();

				if (Stage::get().get_player().request(CharInfoRequestPacket(cid), ExclRequest::USE_WAIT))
					UI::get().emplace<UICharInfo>(cid);

				return;
			}
		}
	}

	void UIPersonalShop::submit(const std::string &text)
	{
		// The server adds our name and sends the line back.
		if (!text.empty())
			MiniRoomChatPacket(text).dispatch();
	}

	void UIPersonalShop::add_line(const std::string &line, Color::Name color)
	{
		bool at_bottom = log_row + LOG_ROWS >= log_rows;

		Text text = Text(Text::Font::A12M, Text::Alignment::LEFT, color, line, LOG_WIDTH, true, 2);
		auto rows = static_cast<int16_t>(std::max(1, (text.height() + LOG_ROW - 3) / LOG_ROW));

		log.push_back({ text, rows });
		log_rows += rows;

		if (log.size() > LOG_LIMIT)
		{
			log_rows -= log.front().rows;
			log_row = std::max<int16_t>(0, log_row - log.front().rows);
			log.erase(log.begin());
		}

		if (at_bottom)
			scroll_to_bottom();
		else
			chat.setrows(log_row, LOG_ROWS, log_rows);
	}

	void UIPersonalShop::scroll_to_bottom()
	{
		log_row = std::max<int16_t>(0, log_rows - LOG_ROWS);
		chat.setrows(log_row, LOG_ROWS, log_rows);
	}

	void UIPersonalShop::refresh_rows()
	{
		rows.clear();

		for (const mini_room::Listing &listing : listings)
		{
			Row row;
			row.name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, ItemData::get(listing.item.itemid).get_name());
			string_format::format_with_ellipsis(row.name, NAME_WIDTH);

			row.price = Text(Text::Font::A12M, Text::Alignment::LEFT, PRICE_COLORS[personal_shop::price_tier(listing.price)],
					personal_shop::price_text(listing));

			rows.push_back(row);
		}

		auto count = static_cast<int16_t>(listings.size());

		first_row = std::clamp<int16_t>(first_row, 0, std::max<int16_t>(0, count - ROWS));
		items.setrows(first_row, ROWS, count);
	}

	void UIPersonalShop::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::PERSONALSHOP);
	}

	void UIPersonalShop::exit()
	{
		// No confirmation either way (FUN_006fb07b).
		MiniRoomLeavePacket().dispatch();

		close();
	}

	UIPersonalShop::RowIcon::RowIcon(int16_t index, int32_t itemid) : index(index), itemid(itemid)
	{}

	InventoryType::Id UIPersonalShop::RowIcon::get_return_tab() const
	{
		return InventoryType::by_item_id(itemid);
	}

	void UIPersonalShop::RowIcon::drop_on_inventory() const
	{
		if (auto shop = UI::get().get_element<UIPersonalShop>())
			shop->take_back(index);
	}

	namespace
	{
		// The sell dialog (FUN_006fe414).
		const Rectangle<int16_t> COUNT_BOX = Rectangle<int16_t>(118, 225, 32, 47);
		const Rectangle<int16_t> PRICE_BOX = Rectangle<int16_t>(118, 225, 52, 67);
		const Rectangle<int16_t> PER_BUNDLE_BOX = Rectangle<int16_t>(118, 225, 72, 87);
		constexpr Point<int16_t> CHECKBOX = Point<int16_t>(11, 92);
		constexpr Point<int16_t> CHECKBOX_SIZE = Point<int16_t>(90, 14);
		constexpr Point<int16_t> BAR = Point<int16_t>(6, 51);
		constexpr Point<int16_t> BT_OK = Point<int16_t>(131, 120);
		constexpr Point<int16_t> BT_CANCEL = Point<int16_t>(180, 120);
	}

	UIShopSell::UIShopSell(InventoryType::Id tab, int16_t slot, int32_t itemid, int16_t quantity) : UIElement(),
			tab(tab), slot(slot), itemid(itemid), quantity(quantity), whole(personal_shop::sells_whole(itemid)), bundled(false)
	{
		nl::node PersonalShop = NxFiles::ui_classic["UIWindow.img"]["PersonalShop"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = PersonalShop["backgrnd2"];
		bar = PersonalShop["bar"];
		checkboxes[0] = Basic["CheckBox"]["0"];
		checkboxes[1] = Basic["CheckBox"]["1"];

		buttons[Buttons::OK] = std::make_unique<MapleButton>(Basic["BtOK2"], BT_OK);
		buttons[Buttons::CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], BT_CANCEL);

		count = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, COUNT_BOX, 4);
		price = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, PRICE_BOX, 10);
		per_bundle = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, PER_BUNDLE_BOX, 4);

		// A whole item sells one at a time; a stack offers all of it.
		count.change_text(std::to_string(whole ? 1 : quantity));
		price.change_text("0");
		per_bundle.change_text(std::to_string(whole ? quantity : 1));

		for (Textfield *field : { &count, &price, &per_bundle })
		{
			field->set_enter_callback([&](std::string) { confirm(); });
			field->set_key_callback(KeyAction::Id::ESCAPE, [&]() { cancel(); });
		}

		if (whole)
			count.set_state(Textfield::State::DISABLED);

		per_bundle.set_state(Textfield::State::DISABLED);
		(whole ? price : count).set_state(Textfield::State::FOCUSED);

		dimension = backgrnd.get_dimensions();
		position = centred(dimension);
	}

	void UIShopSell::draw(float inter) const
	{
		backgrnd.draw(position);

		// Checked, the price is per bundle.
		if (bundled)
			bar.draw(position + BAR);

		checkboxes[bundled].draw(DrawArgument(position + CHECKBOX + Point<int16_t>(0, 1), whole ? 0.5f : 1.0f));

		count.draw(position + TEXT_Y);
		price.draw(position + TEXT_Y);
		per_bundle.draw(position + TEXT_Y);

		UIElement::draw(inter);
	}

	void UIShopSell::update()
	{
		UIElement::update();

		count.update(position);
		price.update(position);
		per_bundle.update(position);
	}

	Cursor::State UIShopSell::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		for (Textfield *field : { &count, &price, &per_bundle })
		{
			Cursor::State fstate = field->send_cursor(cursorpos, clicked);

			if (fstate != Cursor::State::IDLE)
				return fstate;
		}

		Rectangle<int16_t> checkbox = Rectangle<int16_t>(position + CHECKBOX, position + CHECKBOX + CHECKBOX_SIZE);

		if (!whole && checkbox.contains(cursorpos))
		{
			if (clicked)
			{
				bundled = !bundled;
				per_bundle.set_state(bundled ? Textfield::State::NORMAL : Textfield::State::DISABLED);

				return Cursor::State::IDLE;
			}

			return Cursor::State::CANCLICK;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIShopSell::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			cancel();
	}

	UIElement::Type UIShopSell::get_type() const
	{
		return TYPE;
	}

	Button::State UIShopSell::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::OK)
			confirm();
		else
			cancel();

		return Button::State::NORMAL;
	}

	void UIShopSell::confirm()
	{
		personal_shop::SellForm form = { count.get_text(), price.get_text(), per_bundle.get_text(), bundled };
		personal_shop::SellCheck check = personal_shop::check_sale(form, itemid, quantity);

		// A problem keeps the dialog open at the box to fix (FUN_006fe2c3).
		if (check.box != personal_shop::Box::NONE)
		{
			box(check.box).set_state(Textfield::State::FOCUSED);
			notice(check.problem);

			return;
		}

		cancel();

		const personal_shop::Sale &sale = check.sale;

		if (personal_shop::too_pricey(sale.bundles, sale.price))
		{
			notice(TOO_PRICEY);

			return;
		}

		Stage::get().get_player().request(
				ShopPutItemPacket(static_cast<int8_t>(tab), slot, sale.bundles, sale.per_bundle, sale.price),
				ExclRequest::WAIT
		);
	}

	void UIShopSell::cancel()
	{
		for (Textfield *field : { &count, &price, &per_bundle })
		{
			if (field->get_state() == Textfield::State::FOCUSED)
				field->set_state(Textfield::State::NORMAL);
		}

		deactivate();
	}

	Textfield &UIShopSell::box(personal_shop::Box which)
	{
		switch (which)
		{
			case personal_shop::Box::COUNT:
				return count;
			case personal_shop::Box::PER_BUNDLE:
				return per_bundle;
			default:
				return price;
		}
	}

	namespace
	{
		// The sold-items dialog (FUN_006fee6d, FUN_006ff133): four rows of
		// 58, the last one the totals.
		constexpr Point<int16_t> SALES_CLOSE = Point<int16_t>(213, 6);
		constexpr int16_t SALES_SLIDER_X = 212;
		const Range<int16_t> SALES_RANGE = Range<int16_t>(31, 260);
		constexpr int16_t SALES_ROWS = 4;
		constexpr int16_t SALES_PITCH = 58;
		constexpr Point<int16_t> SALES_ICON = Point<int16_t>(8, 75);
		constexpr Point<int16_t> SALES_NAME = Point<int16_t>(50, 32);
		constexpr Point<int16_t> SALES_SECOND = Point<int16_t>(50, 51);
		constexpr Point<int16_t> SALES_THIRD = Point<int16_t>(50, 70);
	}

	UIShopSales::UIShopSales() : UIElement(), first_row(0)
	{
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = NxFiles::ui_classic["UIWindow.img"]["PersonalShop"]["backgrnd3"];
		meso_icon = nl::nx::item["Special"]["0900.img"]["09000001"]["iconRaw"];

		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(Basic["BtClose"], SALES_CLOSE);

		totals = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "Total Sales / Total Received");
		total = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		received = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		slider = Slider(
				Slider::Type::CLASSIC, SALES_RANGE, SALES_SLIDER_X, SALES_ROWS, 0,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;

					if (first_row + shift >= 0 && first_row + shift + SALES_ROWS <= total_rows())
						first_row += shift;
				}
		);

		refresh();

		dimension = backgrnd.get_dimensions();
		position = centred(dimension);
	}

	void UIShopSales::draw(float inter) const
	{
		backgrnd.draw(position);

		for (int16_t k = 0; k < SALES_ROWS; k++)
		{
			auto index = static_cast<size_t>(first_row + k);
			Point<int16_t> row = position + Point<int16_t>(0, SALES_PITCH * k);

			if (index < records.size())
			{
				const Record &record = records[index];

				ItemData::get(record.itemid).get_icon(false).draw(row + SALES_ICON);
				record.name.draw(row + SALES_NAME + TEXT_Y);
				record.buyer.draw(row + SALES_SECOND + TEXT_Y);
				record.units.draw(row + SALES_THIRD + TEXT_Y);
			} else if (index == records.size())
			{
				meso_icon.draw(row + SALES_ICON);
				totals.draw(row + SALES_NAME + TEXT_Y);
				total.draw(row + SALES_SECOND + TEXT_Y);
				received.draw(row + SALES_THIRD + TEXT_Y);
			}
		}

		slider.draw(position);

		UIElement::draw(inter);
	}

	void UIShopSales::remove_cursor()
	{
		UIElement::remove_cursor();

		slider.remove_cursor();
	}

	Cursor::State UIShopSales::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (Cursor::State sstate = slider.send_cursor(cursorpos - position, clicked))
			return sstate;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIShopSales::send_scroll(double yoffset)
	{
		slider.send_scroll(yoffset);
	}

	void UIShopSales::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			deactivate();
	}

	UIElement::Type UIShopSales::get_type() const
	{
		return TYPE;
	}

	void UIShopSales::refresh()
	{
		auto shop = UI::get().get_element<UIPersonalShop>();

		if (!shop)
			return;

		const personal_shop::Sales &sales = shop->get_sales();

		records.clear();

		for (const personal_shop::Sales::Sold &sold : sales.sold)
		{
			Record record;
			record.itemid = sold.itemid;
			record.name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, ItemData::get(sold.itemid).get_name());
			string_format::format_with_ellipsis(record.name, NAME_WIDTH);
			record.buyer = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "Buyer : " + sold.buyer);
			record.units = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
					"Number purchased : " + std::to_string(sold.units));

			records.push_back(record);
		}

		total.change_text(mini_room::meso_text(sales.total) + " mesos.");
		received.change_text(mini_room::meso_text(sales.received) + " mesos.");

		first_row = std::clamp<int16_t>(first_row, 0, static_cast<int16_t>(total_rows() - SALES_ROWS));
		slider.setrows(first_row, SALES_ROWS, total_rows());
	}

	Button::State UIShopSales::button_pressed(uint16_t)
	{
		deactivate();

		return Button::State::NORMAL;
	}

	int16_t UIShopSales::total_rows() const
	{
		// The totals row and one blank after it once the list scrolls
		// (FUN_006ffd6e).
		auto count = static_cast<int16_t>(records.size());

		return count < SALES_ROWS ? SALES_ROWS : count + 2;
	}
}
