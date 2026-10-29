#include "PyramidHandlers.h"

#include "../../Gameplay/Combat/Massacre.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIPyramidResult.h"

namespace ms
{
	void PyramidGaugeHandler::handle(InPacket& recv) const
	{
		if (massacre::session().field != massacre::FIELD_STAGE)
			return;

		massacre::session().count = recv.read_int();
	}

	void PyramidScoreHandler::handle(InPacket& recv) const
	{
		if (massacre::session().field != massacre::FIELD_RESULT)
			return;

		int8_t rank = recv.read_byte();
		int32_t exp = recv.read_int();

		if (auto result = massacre::score(massacre::session(), { rank, exp }))
			UIPyramidResult::show(*result);
	}
}
