#include "MapleTvScreen.h"

#include "../../Graphics/Geometry.h"
#include "../../Util/NxFiles.h"

#include <string>
#include <utility>

namespace ms
{
	namespace
	{
		// The message's screen, from the body's top left (the text builder
		// FUN_006d4ad4, report c056 1.4): 240x90, black for a plain message
		// and clear over a star's or heart's frame.
		constexpr int16_t SCREEN_WIDTH = 240;
		constexpr int16_t SCREEN_HEIGHT = 90;
		// The lines' left and tops, 16 apart, and the names' tops; the
		// receiver's name ends near the right, 4 px a character.
		constexpr int16_t LINES_X = 64;
		constexpr int16_t LINES_Y = 5;
		constexpr int16_t LINE_HEIGHT = 16;
		constexpr int16_t SENDER_X = 7;
		constexpr int16_t RECEIVER_RIGHT = 210;
		constexpr int16_t NAMES_Y = 76;
		// v83 draws a line's cap top about 3 below where it is drawn, we 9
		// (as UIAvatarMegaphone).
		constexpr int16_t TEXT_RAISE = 6;

		// From msgX/Y: where the sender and the receiver stand, facing each
		// other (asm 6d45aa, 6d4618), and the star's and heart's frames
		// (asm 6d4eed, 6d4d4d).
		constexpr Point<int16_t> SENDER_FOOT = Point<int16_t>(28, 77);
		constexpr Point<int16_t> RECEIVER_FOOT = Point<int16_t>(213, 77);
		constexpr Point<int16_t> STAR_FRAME = Point<int16_t>(113, 45);
		constexpr Point<int16_t> HEART_FRAME = Point<int16_t>(119, 45);

		std::optional<MapleTvScreen::Message> current;
		uint32_t generation = 0;
	}

	void MapleTvScreen::show(Message message)
	{
		current = std::move(message);
		generation++;
	}

	void MapleTvScreen::clear()
	{
		current.reset();
		generation++;
	}

	bool MapleTvScreen::enabled(nl::node info)
	{
		return info["MapleTV"].get_integer() != 0;
	}

	MapleTvScreen::Anchors MapleTvScreen::anchors(nl::node info)
	{
		auto value = [&](const char* name)
		{
			return static_cast<int16_t>(info[name].get_integer());
		};

		return {
			Point<int16_t>(value("MapleTVmsgX"), value("MapleTVmsgY")),
			Point<int16_t>(value("MapleTVadX"), value("MapleTVadY"))
		};
	}

	Point<int16_t> MapleTvScreen::body_position(Anchors anchors)
	{
		return anchors.message + Point<int16_t>(0, 0x5a);
	}

	Point<int16_t> MapleTvScreen::ad_position(Anchors anchors)
	{
		return anchors.ad + Point<int16_t>(0, 0xb4);
	}

	MapleTvScreen::MapleTvScreen() : on(false), at{}, seen(0), showing(false), type(0), receiver_x(0) {}

	MapleTvScreen::MapleTvScreen(nl::node info, int32_t media)
			: on(true), at(anchors(info)), seen(generation), showing(false), type(0), receiver_x(0)
	{
		nl::node src = NxFiles::ui_classic["MapleTV.img"];

		ad = src["TVmedia"][std::to_string(media)];

		// The body turns off once and holds its last frame, the logo
		// (CNpc::Init FUN_006d089a).
		body = src["TVoff"];
		body.play(Animation::End::HOLD);

		if (current)
			show_message(*current);
	}

	void MapleTvScreen::show_message(const Message& message)
	{
		nl::node src = NxFiles::ui_classic["MapleTV.img"];

		// The text builder clears the body, so TVon is cut after a frame
		// and the screen is the message's (asm 6d4c75).
		showing = true;
		type = message.type;
		body = Animation();

		switch (type)
		{
			case 1:
				frame = src["TVchat1"];
				break;
			case 2:
				frame = src["TVchat2"];
				break;
			default:
				frame = Animation();
				break;
		}

		sender_look.emplace(message.sender_look);

		if (message.receiver_look)
			receiver_look.emplace(*message.receiver_look);
		else
			receiver_look.reset();

		// Arial 12 in white (Init's font, asm 46341a call).
		sender = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, message.sender);
		receiver = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, message.receiver);
		receiver_x = static_cast<int16_t>(RECEIVER_RIGHT - 4 * static_cast<int16_t>(message.receiver.size()));

		for (size_t i = 0; i < lines.size(); i++)
			lines[i] = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, message.lines[i]);
	}

	void MapleTvScreen::turn_off()
	{
		// recv 342 (asm 6d4679-6d4a53): the message goes and TVoff plays
		// once.
		showing = false;
		frame = Animation();
		sender_look.reset();
		receiver_look.reset();

		body = NxFiles::ui_classic["MapleTV.img"]["TVoff"];
		body.play(Animation::End::HOLD);
	}

	void MapleTvScreen::update()
	{
		if (!on)
			return;

		if (seen != generation)
		{
			seen = generation;

			if (current)
				show_message(*current);
			else if (showing)
				turn_off();
		}

		ad.update();
		body.update();
		frame.update();
	}

	void MapleTvScreen::draw(Point<int16_t> foot, float alpha) const
	{
		if (!on)
			return;

		ad.draw(foot + ad_position(at), alpha);

		// The star's and heart's frames lie behind the body (z -1).
		Point<int16_t> message = foot + at.message;
		frame.draw(message + (type == 2 ? HEART_FRAME : STAR_FRAME), alpha);
		body.draw(foot + body_position(at), alpha);

		if (!showing)
			return;

		Point<int16_t> screen = message + Point<int16_t>(0, 1);

		if (type == 0)
			ColorBox(SCREEN_WIDTH, SCREEN_HEIGHT, Color::Name::BLACK, 1.0f).draw(DrawArgument(screen));

		for (size_t i = 0; i < lines.size(); i++)
			lines[i].draw(screen + Point<int16_t>(LINES_X, LINES_Y + LINE_HEIGHT * static_cast<int16_t>(i) - TEXT_RAISE));

		sender.draw(screen + Point<int16_t>(SENDER_X, NAMES_Y - TEXT_RAISE));
		receiver.draw(screen + Point<int16_t>(receiver_x, NAMES_Y - TEXT_RAISE));

		if (sender_look)
			sender_look->draw(message + SENDER_FOOT, true, Stance::Id::STAND1, Expression::Id::DEFAULT);

		if (receiver_look)
			receiver_look->draw(message + RECEIVER_FOOT, false, Stance::Id::STAND1, Expression::Id::DEFAULT);
	}
}
