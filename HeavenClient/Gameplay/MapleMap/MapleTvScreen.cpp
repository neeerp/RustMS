#include "MapleTvScreen.h"

#include "../../Util/NxFiles.h"

#include <string>

namespace ms
{
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

	MapleTvScreen::MapleTvScreen() : on(false), at{} {}

	MapleTvScreen::MapleTvScreen(nl::node info, int32_t media) : on(true), at(anchors(info))
	{
		nl::node src = NxFiles::ui_classic["MapleTV.img"];

		ad = src["TVmedia"][std::to_string(media)];

		// The body turns off once and holds its last frame, the logo
		// (CNpc::Init FUN_006d089a).
		body = src["TVoff"];
		body.play(Animation::End::HOLD);
	}

	void MapleTvScreen::update()
	{
		if (!on)
			return;

		ad.update();
		body.update();
	}

	void MapleTvScreen::draw(Point<int16_t> foot, float alpha) const
	{
		if (!on)
			return;

		ad.draw(foot + ad_position(at), alpha);
		body.draw(foot + body_position(at), alpha);
	}
}
